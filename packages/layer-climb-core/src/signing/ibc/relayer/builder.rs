// at a high-level, the builder is essentially going from
// (1) a list of clients
// (2) a list of paths
// (3) an optional lightweight cache of serializable data needed to make this all work
//
// to: a relayer with merely a list of client infos
// "client infos" is a collection of clients and their associated data
// where "associated data" is, for example, data needed to do a reverse lookup from an ibc chain event
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    ibc_types::{
        IbcChannelId, IbcChannelOrdering, IbcChannelVersion, IbcClientId, IbcConnectionId,
        IbcPortId,
    },
    prelude::*,
    signing::ibc::{IbcChannelHandshake, IbcConnectionHandshake},
    transaction::SequenceStrategyKind,
};
use futures::{future::Either, pin_mut};
use serde::{Deserialize, Serialize};

use super::{
    ClientInfo, ClientInfoChannel, ClientUpdate, IbcRelayer, IbcRelayerGasSimulationMultipliers,
    Side,
};

pub struct IbcRelayerBuilder {
    clients: HashMap<ChainId, SigningClient>,
    paths: Vec<IbcPath>,
    simulation_gas_multipliers: IbcRelayerGasSimulationMultipliers,
    inner_log_ok: Arc<dyn Fn(String) + Send + Sync + 'static>,
    inner_log_err: Arc<dyn Fn(String) + Send + Sync + 'static>,
    cache: Arc<Mutex<IbcCache>>,
    client_infos: Arc<Mutex<HashMap<IbcCacheChainKey, ClientInfo>>>,
    client_infos_updating: Arc<Mutex<Vec<Arc<ClientInfo>>>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct IbcPath {
    pub chain_id_1: ChainId,
    pub chain_id_2: ChainId,
    pub port_id_1: IbcPortId,
    pub port_id_2: IbcPortId,
    pub channel_version: IbcChannelVersion,
    pub channel_ordering: IbcChannelOrdering,
}

impl IbcRelayerBuilder {
    pub fn new(
        clients: Vec<SigningClient>,
        mut paths: Vec<IbcPath>,
        // if None, IbcRelayerGasSimulationMultipliers::default() will be used
        gas_simulation_multipliers: Option<IbcRelayerGasSimulationMultipliers>,
        log_ok: impl Fn(String) + Send + Sync + 'static,
        log_err: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        // Dedupe paths and make sure they are sorted
        // this will also ensure that we can look up a ClientInfo by the chain_id_1 and chain_id_2
        for path in &mut paths {
            if path.chain_id_1 > path.chain_id_2 {
                std::mem::swap(&mut path.chain_id_1, &mut path.chain_id_2);
                std::mem::swap(&mut path.port_id_1, &mut path.port_id_2);
            }
        }
        let mut found = HashSet::new();
        paths.retain(|p| found.insert(p.clone()));

        Self {
            inner_log_ok: Arc::new(log_ok),
            inner_log_err: Arc::new(log_err),
            clients: clients
                .into_iter()
                .map(|c| (c.chain_id().clone(), c))
                .collect(),
            paths,
            simulation_gas_multipliers: gas_simulation_multipliers.unwrap_or_default(),
            cache: Arc::new(Mutex::new(IbcCache::default())),
            client_infos: Arc::new(Mutex::new(HashMap::new())),
            client_infos_updating: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // call prep_cache() on the builder, optionally stash the cache, and then build()
    pub async fn build(self) -> Result<IbcRelayer> {
        let client_infos: Vec<Arc<ClientInfo>> =
            std::mem::take(&mut *self.client_infos.lock().unwrap())
                .into_values()
                .map(Arc::new)
                .collect();

        if client_infos.is_empty() {
            return Err(anyhow::anyhow!("No client infos found"));
        }

        Ok(IbcRelayer {
            simulation_gas_multipliers: self.simulation_gas_multipliers,
            inner_log_ok: self.inner_log_ok,
            inner_log_err: self.inner_log_err,
            client_infos,
        })
    }

    // this will create/update the cache and return it, so you can pass it back in next time
    // it *must* be run before build()
    //
    // note that this relayer is essentially focused on dev ergonomics for "relay over these ports"
    // and has not been tested with a cache created by outside configuration yet (though it should work)
    pub async fn prep_cache(&self, initial_cache: Option<IbcCache>) -> Result<IbcCache> {
        let fut1 = async { self.prep_cache_inner(initial_cache).await };
        pin_mut!(fut1);

        let fut2 = async { self.check_client_updates_while_prepping().await };
        pin_mut!(fut2);

        let resp = futures::future::select(fut1, fut2).await;

        match resp {
            Either::Left((x, _)) => x,
            Either::Right((y, _)) => {
                y?;
                Err(anyhow::anyhow!("unreachable"))
            }
        }
    }

    async fn check_client_updates_while_prepping(&self) -> Result<()> {
        loop {
            // this is not very efficient, but, it's not _that_ bad... and there's a sleep in between, and it's only while prepping the cache
            // and the whole point is client infos may change while we're prepping the cache
            let client_infos_updating = {
                let lock = self.client_infos_updating.lock().unwrap();
                let v = &*lock;
                v.clone()
            };

            for client_info in client_infos_updating {
                // don't even attempt it if the sequence strategy is query, will likely get account sequence errors
                if !matches!(
                    client_info.signing_client_1.sequence_strategy_kind(),
                    SequenceStrategyKind::Query
                ) {
                    let current_height =
                        client_info.signing_client_1.querier.block_height().await?;
                    if client_info
                        .is_past_update_height(Side::One, current_height)
                        .await?
                    {
                        self.update_ibc_client(&client_info, Side::One).await?;
                    }
                }

                // don't even attempt it if the sequence strategy is query, will likely get account sequence errors
                if !matches!(
                    client_info.signing_client_2.sequence_strategy_kind(),
                    SequenceStrategyKind::Query
                ) {
                    let current_height =
                        client_info.signing_client_2.querier.block_height().await?;
                    if client_info
                        .is_past_update_height(Side::Two, current_height)
                        .await?
                    {
                        self.update_ibc_client(&client_info, Side::Two).await?;
                    }
                }
            }

            futures_timer::Delay::new(Duration::from_secs(1)).await;
        }
    }

    async fn update_ibc_client(&self, client_info: &ClientInfo, side: Side) -> Result<()> {
        let log_ok = self.inner_log_ok.clone();
        client_info
            .update(side, &self.simulation_gas_multipliers, move |s| log_ok(s))
            .await
    }

    async fn prep_cache_inner(&self, initial_cache: Option<IbcCache>) -> Result<IbcCache> {
        {
            let mut lock = self.cache.lock().unwrap();
            if let Some(cache) = initial_cache {
                *lock = cache;
            }

            lock.chains.retain(|k, _| {
                self.clients.contains_key(&k.chain_id_1) && self.clients.contains_key(&k.chain_id_2)
            })
        }

        for path in &self.paths {
            let client_1 = self.get_signing_client(&path.chain_id_1)?;
            let client_2 = self.get_signing_client(&path.chain_id_2)?;
            let ibc_chain_cache_key = IbcCacheChainKey {
                chain_id_1: path.chain_id_1.clone(),
                chain_id_2: path.chain_id_2.clone(),
            };

            let mut ibc_client_cache = self.get_ibc_client_cache(&ibc_chain_cache_key);
            if let Ok(c) = ibc_client_cache.as_ref() {
                self.log_ok(format!(
                    "client {} for chain {} exists in cache, checking for staleness via update...",
                    c.ibc_client_id_1, path.chain_id_1
                ));
                let mut tx_builder = client_1.tx_builder();
                if let Some(gas_simulation_multiplier) =
                    self.simulation_gas_multipliers.update_client_1
                {
                    tx_builder.set_gas_simulate_multiplier(gas_simulation_multiplier);
                }
                if client_1
                    .ibc_update_client(
                        &c.ibc_client_id_1,
                        &client_2.querier,
                        None,
                        Some(tx_builder),
                    )
                    .await
                    .is_err()
                {
                    self.log_ok(format!(
                        "client {} for chain {} exists in cache, but is stale",
                        c.ibc_client_id_1, path.chain_id_1
                    ));
                    ibc_client_cache = Err(anyhow::anyhow!("Failed to update clients"));
                } else {
                    let mut tx_builder = client_2.tx_builder();
                    if let Some(gas_simulation_multiplier) =
                        self.simulation_gas_multipliers.update_client_2
                    {
                        tx_builder.set_gas_simulate_multiplier(gas_simulation_multiplier);
                    }
                    self.log_ok(format!("client {} for chain {} exists in cache, checking for staleness via update...", c.ibc_client_id_2, path.chain_id_2));
                    if client_2
                        .ibc_update_client(
                            &c.ibc_client_id_2,
                            &client_1.querier,
                            None,
                            Some(tx_builder),
                        )
                        .await
                        .is_err()
                    {
                        self.log_ok(format!(
                            "client {} for chain {} exists in cache, but is stale",
                            c.ibc_client_id_2, path.chain_id_2
                        ));
                        ibc_client_cache = Err(anyhow::anyhow!("Failed to update clients"));
                    }
                }
            }

            let (conn_handshake, channel_handshake) = match ibc_client_cache {
                Err(_) => {
                    self.log_ok(format!(
                        "Creating brand new clients for path {} <-> {}",
                        path.chain_id_1, path.chain_id_2
                    ));
                    let conn_handshake = client_1
                        .ibc_connection_handshake(
                            &client_2,
                            None,
                            None,
                            self.simulation_gas_multipliers.connection_handshake.clone(),
                            |s| self.log_ok(s),
                        )
                        .await?;

                    {
                        let mut lock = self.cache.lock().unwrap();
                        let mut ibc_connections = HashMap::new();
                        ibc_connections.insert(
                            IbcCacheConnectionKey {
                                connection_id_1: conn_handshake.connection_id.clone(),
                                connection_id_2: conn_handshake.counterparty_connection_id.clone(),
                            },
                            IbcChannelCache {
                                ibc_channels: HashMap::new(),
                            },
                        );

                        lock.chains.insert(
                            ibc_chain_cache_key.clone(),
                            IbcClientCache {
                                ibc_client_id_1: conn_handshake.client_id.clone(),
                                ibc_client_id_2: conn_handshake.counterparty_client_id.clone(),
                                ibc_connections,
                            },
                        );
                    }

                    anyhow::Ok((conn_handshake, None))
                }
                Ok(mut ibc_client_cache) => {
                    self.log_ok(format!(
                        "Clients already exist for path {} <-> {}",
                        path.chain_id_1, path.chain_id_2
                    ));
                    let entry =
                        ibc_client_cache
                            .ibc_connections
                            .iter()
                            .find_map(|(connection_ids, v)| {
                                // INVARIANT: port<->port is distinct, on precisely one connection in the cache
                                match v
                                    .ibc_channels
                                    .get(&IbcCacheChannelKey {
                                        port_id_1: path.port_id_1.clone(),
                                        port_id_2: path.port_id_2.clone(),
                                    })
                                    .cloned()
                                {
                                    Some((
                                        channel_id,
                                        counterparty_channel_id,
                                        channel_version,
                                    )) => {
                                        if channel_version == path.channel_version {
                                            Some((
                                                connection_ids.clone(),
                                                IbcChannelHandshake {
                                                    channel_id,
                                                    counterparty_channel_id,
                                                },
                                            ))
                                        } else {
                                            None
                                        }
                                    }
                                    None => None,
                                }
                            });
                    match entry {
                        None => {
                            let ibc_client_id_1 = ibc_client_cache.ibc_client_id_1.clone();
                            let ibc_client_id_2 = ibc_client_cache.ibc_client_id_2.clone();
                            self.log_ok(format!(
                                "Creating new connection for path {} <-> {} over clients {},{}",
                                path.chain_id_1, path.chain_id_2, ibc_client_id_1, ibc_client_id_2
                            ));
                            let conn_handshake = client_1
                                .ibc_connection_handshake(
                                    &client_2,
                                    Some(ibc_client_cache.ibc_client_id_1.clone()),
                                    Some(ibc_client_cache.ibc_client_id_2.clone()),
                                    self.simulation_gas_multipliers.connection_handshake.clone(),
                                    |s| self.log_ok(s),
                                )
                                .await?;

                            {
                                let mut lock = self.cache.lock().unwrap();
                                ibc_client_cache.ibc_connections.insert(
                                    IbcCacheConnectionKey {
                                        connection_id_1: conn_handshake.connection_id.clone(),
                                        connection_id_2: conn_handshake
                                            .counterparty_connection_id
                                            .clone(),
                                    },
                                    IbcChannelCache {
                                        ibc_channels: HashMap::new(),
                                    },
                                );

                                lock.chains
                                    .insert(ibc_chain_cache_key.clone(), ibc_client_cache.clone());
                            }

                            Ok((conn_handshake, None))
                        }
                        Some((connection_ids, channel_handshake)) => {
                            let conn_handshake = IbcConnectionHandshake {
                                client_id: ibc_client_cache.ibc_client_id_1.clone(),
                                counterparty_client_id: ibc_client_cache.ibc_client_id_2.clone(),
                                connection_id: connection_ids.connection_id_1.clone(),
                                counterparty_connection_id: connection_ids.connection_id_2.clone(),
                            };

                            Ok((conn_handshake, Some(channel_handshake)))
                        }
                    }
                }
            }?;

            let IbcConnectionHandshake {
                connection_id: ibc_connection_id_1,
                counterparty_connection_id: ibc_connection_id_2,
                client_id: ibc_client_id_1,
                counterparty_client_id: ibc_client_id_2,
            } = &conn_handshake;

            let channel_handshake = match channel_handshake {
                None => {
                    self.log_ok(format!(
                        "Creating channel over connection {}:{} <-> {}:{}, version {}",
                        path.chain_id_1,
                        ibc_connection_id_1,
                        path.chain_id_2,
                        ibc_connection_id_2,
                        path.channel_version
                    ));

                    let channel_handshake = client_1
                        .ibc_channel_handshake(
                            &client_2,
                            &path.port_id_1,
                            &path.port_id_2,
                            &path.channel_version,
                            path.channel_ordering,
                            &conn_handshake,
                            self.simulation_gas_multipliers.channel_handshake.clone(),
                            |s| self.log_ok(s),
                        )
                        .await?;

                    {
                        let mut lock = self.cache.lock().unwrap();
                        let ibc_client_cache = lock.chains.get_mut(&ibc_chain_cache_key).unwrap();
                        let ibc_connection_cache = ibc_client_cache
                            .ibc_connections
                            .get_mut(&IbcCacheConnectionKey {
                                connection_id_1: ibc_connection_id_1.clone(),
                                connection_id_2: ibc_connection_id_2.clone(),
                            })
                            .unwrap();
                        ibc_connection_cache.ibc_channels.insert(
                            IbcCacheChannelKey {
                                port_id_1: path.port_id_1.clone(),
                                port_id_2: path.port_id_2.clone(),
                            },
                            (
                                channel_handshake.channel_id.clone(),
                                channel_handshake.counterparty_channel_id.clone(),
                                path.channel_version.clone(),
                            ),
                        );
                    }

                    channel_handshake
                }
                Some(channel_handshake) => {
                    self.log_ok(format!(
                        "Channel already exists over {}:{}:{} <-> {}:{}:{}, version {}",
                        path.chain_id_1,
                        ibc_connection_id_1,
                        channel_handshake.channel_id,
                        path.chain_id_2,
                        ibc_connection_id_2,
                        channel_handshake.counterparty_channel_id,
                        path.channel_version
                    ));

                    channel_handshake
                }
            };

            // create client info and add it to the collection
            // the collection is stored on self behind a mutex so we can update clients
            // as we create others
            {
                let client_info_channel = ClientInfoChannel {
                    channel_id_1: channel_handshake.channel_id.clone(),
                    channel_id_2: channel_handshake.counterparty_channel_id.clone(),
                    port_id_1: path.port_id_1.clone(),
                    port_id_2: path.port_id_2.clone(),
                    channel_version: path.channel_version.clone(),
                    channel_ordering: path.channel_ordering,
                };

                let has_client_info = {
                    let mut lock = self.client_infos.lock().unwrap();
                    match lock.get_mut(&ibc_chain_cache_key) {
                        Some(client_info) => {
                            // for the updating list, we don't care about the channels, it can stay as is
                            // just need to push new channels to the real list
                            client_info.channels.push(client_info_channel.clone());
                            true
                        }
                        None => false,
                    }
                };

                if !has_client_info {
                    let client_state_1 = client_1
                        .querier
                        .ibc_client_state(ibc_client_id_1, None)
                        .await?;
                    let client_state_2 = client_2
                        .querier
                        .ibc_client_state(ibc_client_id_2, None)
                        .await?;

                    let trusting_period_1 = client_state_1
                        .trusting_period
                        .context("No trusting period found")?;
                    let trusting_period_2 = client_state_2
                        .trusting_period
                        .context("No trusting period found")?;

                    let client_info = ClientInfo {
                        signing_client_1: client_1,
                        signing_client_2: client_2,
                        ibc_client_id_1: ibc_client_id_1.clone(),
                        ibc_client_id_2: ibc_client_id_2.clone(),
                        trusting_period_1: Duration::new(
                            trusting_period_1.seconds as u64,
                            trusting_period_1.nanos as u32,
                        ),
                        trusting_period_2: Duration::new(
                            trusting_period_2.seconds as u64,
                            trusting_period_2.nanos as u32,
                        ),
                        update_1: ClientUpdate::default(),
                        update_2: ClientUpdate::default(),
                        connection_id_1: ibc_connection_id_1.clone(),
                        connection_id_2: ibc_connection_id_2.clone(),
                        channels: vec![client_info_channel],
                    };

                    {
                        let mut lock = self.client_infos_updating.lock().unwrap();
                        // to avoid ambiguity of making ClientInfo Clone, this is the only place that would benefit
                        // so just do the dirty work :P
                        let client_info_for_update = ClientInfo {
                            signing_client_1: client_info.signing_client_1.clone(),
                            signing_client_2: client_info.signing_client_2.clone(),
                            ibc_client_id_1: client_info.ibc_client_id_1.clone(),
                            ibc_client_id_2: client_info.ibc_client_id_2.clone(),
                            trusting_period_1: client_info.trusting_period_1,
                            trusting_period_2: client_info.trusting_period_2,
                            update_1: ClientUpdate::default(),
                            update_2: ClientUpdate::default(),
                            connection_id_1: client_info.connection_id_1.clone(),
                            connection_id_2: client_info.connection_id_2.clone(),
                            channels: client_info.channels.clone(),
                        };

                        lock.push(Arc::new(client_info_for_update));
                    }

                    {
                        let mut lock = self.client_infos.lock().unwrap();
                        lock.insert(ibc_chain_cache_key.clone(), client_info);
                    }
                }
            };
        }

        Ok(self.cache.lock().unwrap().clone())
    }

    fn get_signing_client(&self, chain_id: &ChainId) -> Result<SigningClient> {
        self.clients
            .get(chain_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No signing client found for chain {}", chain_id))
    }

    fn get_ibc_client_cache(&self, chain_key: &IbcCacheChainKey) -> Result<IbcClientCache> {
        let cache = self.cache.lock().unwrap();
        cache.chains.get(chain_key).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "No chain cache found for chains {} and {}",
                chain_key.chain_id_1,
                chain_key.chain_id_2
            )
        })
    }

    fn log_ok(&self, s: String) {
        (self.inner_log_ok)(s);
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct IbcCache {
    pub chains: HashMap<IbcCacheChainKey, IbcClientCache>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct IbcCacheChainKey {
    pub chain_id_1: ChainId,
    pub chain_id_2: ChainId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbcClientCache {
    pub ibc_client_id_1: IbcClientId,
    pub ibc_client_id_2: IbcClientId,
    pub ibc_connections: HashMap<IbcCacheConnectionKey, IbcChannelCache>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct IbcCacheConnectionKey {
    pub connection_id_1: IbcConnectionId,
    pub connection_id_2: IbcConnectionId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbcChannelCache {
    pub ibc_channels: HashMap<IbcCacheChannelKey, (IbcChannelId, IbcChannelId, IbcChannelVersion)>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct IbcCacheChannelKey {
    pub port_id_1: IbcPortId,
    pub port_id_2: IbcPortId,
}
// JSON serializers for cache keys
impl Serialize for IbcCacheChainKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}|{}", self.chain_id_1, self.chain_id_2))
    }
}

impl<'de> Deserialize<'de> for IbcCacheChainKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom("invalid IbcCacheClientKey format"));
        }
        Ok(IbcCacheChainKey {
            chain_id_1: ChainId::new(parts[0].to_string()),
            chain_id_2: ChainId::new(parts[1].to_string()),
        })
    }
}

impl Serialize for IbcCacheConnectionKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!(
            "{}|{}",
            self.connection_id_1, self.connection_id_2
        ))
    }
}

impl<'de> Deserialize<'de> for IbcCacheConnectionKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom(
                "invalid IbcCacheConnectionKey format",
            ));
        }
        Ok(IbcCacheConnectionKey {
            connection_id_1: IbcConnectionId::new(parts[0].to_string()),
            connection_id_2: IbcConnectionId::new(parts[1].to_string()),
        })
    }
}

impl Serialize for IbcCacheChannelKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}|{}", self.port_id_1, self.port_id_2))
    }
}

impl<'de> Deserialize<'de> for IbcCacheChannelKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom(
                "invalid IbcCacheChannelKey format",
            ));
        }
        Ok(IbcCacheChannelKey {
            port_id_1: IbcPortId::new(parts[0].to_string()),
            port_id_2: IbcPortId::new(parts[1].to_string()),
        })
    }
}
