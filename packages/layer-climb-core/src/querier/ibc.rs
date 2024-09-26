use crate::{
    ibc_types::{IbcChannelId, IbcClientId, IbcConnectionId, IbcPortId},
    prelude::*,
};

use super::{
    abci::{AbciProofKind, AbciProofReq},
    basic::{BlockHeaderReq, BlockHeightReq, StakingParamsReq},
};

impl QueryClient {
    pub async fn ibc_connection_proofs(
        &self,
        proof_height: layer_climb_proto::RevisionHeight,
        client_id: &IbcClientId,
        connection_id: &IbcConnectionId,
    ) -> Result<IbcConnectionProofs> {
        self.run_with_middleware(IbcConnectionProofsReq {
            proof_height,
            client_id: client_id.clone(),
            connection_id: connection_id.clone(),
        })
        .await
    }

    pub async fn ibc_channel_proofs(
        &self,
        proof_height: layer_climb_proto::RevisionHeight,
        channel_id: &IbcChannelId,
        port_id: &IbcPortId,
    ) -> Result<IbcChannelProofs> {
        self.run_with_middleware(IbcChannelProofsReq {
            proof_height,
            channel_id: channel_id.clone(),
            port_id: port_id.clone(),
        })
        .await
    }

    pub async fn ibc_client_state(
        &self,
        ibc_client_id: &IbcClientId,
        height: Option<u64>,
    ) -> Result<layer_climb_proto::ibc::light_client::ClientState> {
        self.run_with_middleware(IbcClientStateReq {
            ibc_client_id: ibc_client_id.clone(),
            height,
        })
        .await
    }

    pub async fn ibc_connection(
        &self,
        connection_id: &IbcConnectionId,
        height: Option<u64>,
    ) -> Result<layer_climb_proto::ibc::connection::ConnectionEnd> {
        self.run_with_middleware(IbcConnectionReq {
            connection_id: connection_id.clone(),
            height,
        })
        .await
    }

    pub async fn ibc_connection_consensus_state(
        &self,
        connection_id: &IbcConnectionId,
        consensus_height: Option<layer_climb_proto::RevisionHeight>,
        height: Option<u64>,
    ) -> Result<layer_climb_proto::Any> {
        self.run_with_middleware(IbcConnectionConsensusStateReq {
            connection_id: connection_id.clone(),
            consensus_height,
            height,
        })
        .await
    }

    pub async fn ibc_channel(
        &self,
        channel_id: &IbcChannelId,
        port_id: &IbcPortId,
        height: Option<u64>,
    ) -> Result<layer_climb_proto::ibc::channel::Channel> {
        self.run_with_middleware(IbcChannelReq {
            channel_id: channel_id.clone(),
            port_id: port_id.clone(),
            height,
        })
        .await
    }

    pub async fn ibc_create_client_consensus_state(
        &self,
        trusting_period_secs: Option<u64>,
    ) -> Result<(
        layer_climb_proto::ibc::light_client::ClientState,
        layer_climb_proto::ibc::light_client::ConsensusState,
    )> {
        self.run_with_middleware(IbcCreateClientConsensusStateReq {
            trusting_period_secs,
        })
        .await
    }
}

#[derive(Clone, Debug)]
struct IbcConnectionProofsReq {
    pub proof_height: layer_climb_proto::RevisionHeight,
    pub client_id: IbcClientId,
    pub connection_id: IbcConnectionId,
}

impl QueryRequest for IbcConnectionProofsReq {
    type QueryResponse = IbcConnectionProofs;

    async fn request(&self, client: QueryClient) -> Result<IbcConnectionProofs> {
        let IbcConnectionProofsReq {
            proof_height,
            client_id,
            connection_id,
        } = self;

        let query_height = proof_height.revision_height - 1;

        let connection = IbcConnectionReq {
            connection_id: connection_id.clone(),
            height: Some(query_height),
        }
        .request(client.clone())
        .await?;
        let connection_proof = AbciProofReq {
            kind: AbciProofKind::IbcConnection {
                connection_id: connection_id.clone(),
            },
            height: query_height,
        }
        .request(client.clone())
        .await?
        .proof;

        let client_state = IbcClientStateReq {
            ibc_client_id: client_id.clone(),
            height: Some(query_height),
        }
        .request(client.clone())
        .await?;
        let client_state_proof = AbciProofReq {
            kind: AbciProofKind::IbcClientState {
                client_id: client_id.clone(),
            },
            height: query_height,
        }
        .request(client.clone())
        .await?
        .proof;

        let consensus_height = *client_state
            .latest_height
            .as_ref()
            .context("missing client state latest height")?;

        let consensus_proof = AbciProofReq {
            kind: AbciProofKind::IbcConsensus {
                client_id: client_id.clone(),
                height: consensus_height,
            },
            height: query_height,
        }
        .request(client.clone())
        .await?
        .proof;

        if client_state_proof.is_empty() {
            bail!("missing client state proof");
        }
        if connection_proof.is_empty() {
            bail!("missing connection proof");
        }
        if consensus_proof.is_empty() {
            bail!("missing consensus proof");
        }

        Ok(IbcConnectionProofs {
            proof_height: *proof_height,
            consensus_height,
            query_height,
            connection,
            connection_proof,
            client_state_proof,
            consensus_proof,
            client_state,
        })
    }
}

#[derive(Clone, Debug)]
struct IbcChannelProofsReq {
    pub proof_height: layer_climb_proto::RevisionHeight,
    pub channel_id: IbcChannelId,
    pub port_id: IbcPortId,
}

impl QueryRequest for IbcChannelProofsReq {
    type QueryResponse = IbcChannelProofs;

    async fn request(&self, client: QueryClient) -> Result<IbcChannelProofs> {
        let IbcChannelProofsReq {
            proof_height,
            channel_id,
            port_id,
        } = self;

        let query_height = proof_height.revision_height - 1;

        let channel = IbcChannelReq {
            channel_id: channel_id.clone(),
            port_id: port_id.clone(),
            height: Some(query_height),
        }
        .request(client.clone())
        .await?;
        let channel_proof = AbciProofReq {
            kind: AbciProofKind::IbcChannel {
                channel_id: channel_id.clone(),
                port_id: port_id.clone(),
            },
            height: query_height,
        }
        .request(client)
        .await?
        .proof;

        Ok(IbcChannelProofs {
            proof_height: *proof_height,
            query_height,
            channel,
            channel_proof,
        })
    }
}

#[derive(Clone, Debug)]
struct IbcClientStateReq {
    pub ibc_client_id: IbcClientId,
    pub height: Option<u64>,
}

impl QueryRequest for IbcClientStateReq {
    type QueryResponse = layer_climb_proto::ibc::light_client::ClientState;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::ibc::light_client::ClientState> {
        let IbcClientStateReq {
            ibc_client_id,
            height,
        } = self;

        let mut req =
            tonic::Request::new(layer_climb_proto::ibc::client::QueryClientStateRequest {
                client_id: ibc_client_id.to_string(),
            });

        apply_grpc_height(&mut req, *height)?;

        let mut query_client = layer_climb_proto::ibc::client::query_client::QueryClient::new(
            client.grpc_channel.clone(),
        );
        let resp: layer_climb_proto::ibc::client::QueryClientStateResponse = query_client
            .client_state(req)
            .await
            .map(|res| res.into_inner())
            .context("couldn't get client state")?;

        let client_state = resp
            .client_state
            .map(|client_state| match client_state.type_url.as_str() {
                "/ibc.lightclients.tendermint.v1.ClientState" => {
                    layer_climb_proto::ibc::light_client::ClientState::decode(
                        client_state.value.as_slice(),
                    )
                    .map_err(|e| e.into())
                }
                _ => Err(anyhow::anyhow!(
                    "unsupported client state type: {}",
                    client_state.type_url
                )),
            })
            .transpose()?
            .context("missing client state")?;

        Ok(client_state)
    }
}

#[derive(Clone, Debug)]
struct IbcConnectionReq {
    pub connection_id: IbcConnectionId,
    pub height: Option<u64>,
}

impl QueryRequest for IbcConnectionReq {
    type QueryResponse = layer_climb_proto::ibc::connection::ConnectionEnd;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::ibc::connection::ConnectionEnd> {
        let IbcConnectionReq {
            connection_id,
            height,
        } = self;

        let mut req =
            tonic::Request::new(layer_climb_proto::ibc::connection::QueryConnectionRequest {
                connection_id: connection_id.to_string(),
            });

        apply_grpc_height(&mut req, *height)?;

        let mut query_client = layer_climb_proto::ibc::connection::query_client::QueryClient::new(
            client.grpc_channel.clone(),
        );

        query_client
            .connection(req)
            .await
            .map(|res| res.into_inner())
            .context("couldn't get connection")?
            .connection
            .context("missing connection")
    }
}

#[derive(Clone, Debug)]
struct IbcConnectionConsensusStateReq {
    pub connection_id: IbcConnectionId,
    pub consensus_height: Option<layer_climb_proto::RevisionHeight>,
    pub height: Option<u64>,
}

impl QueryRequest for IbcConnectionConsensusStateReq {
    type QueryResponse = layer_climb_proto::Any;

    async fn request(&self, client: QueryClient) -> Result<layer_climb_proto::Any> {
        let IbcConnectionConsensusStateReq {
            connection_id,
            consensus_height,
            height,
        } = self;

        let mut query_client = layer_climb_proto::ibc::connection::query_client::QueryClient::new(
            client.grpc_channel.clone(),
        );

        let consensus_height = match consensus_height {
            Some(h) => *h,
            None => layer_climb_proto::RevisionHeight {
                revision_number: client.chain_config.ibc_client_revision()?,
                revision_height: match height {
                    Some(h) => *h,
                    None => BlockHeightReq {}.request(client).await?,
                },
            },
        };

        let mut req = tonic::Request::new(
            layer_climb_proto::ibc::connection::QueryConnectionConsensusStateRequest {
                connection_id: connection_id.to_string(),
                revision_number: consensus_height.revision_number,
                revision_height: consensus_height.revision_height,
            },
        );

        apply_grpc_height(&mut req, *height)?;

        query_client
            .connection_consensus_state(req)
            .await
            .map(|res| res.into_inner())
            .context("couldn't get consensus state")?
            .consensus_state
            .context("missing consensus state")
    }
}

#[derive(Clone, Debug)]
struct IbcChannelReq {
    pub channel_id: IbcChannelId,
    pub port_id: IbcPortId,
    pub height: Option<u64>,
}

impl QueryRequest for IbcChannelReq {
    type QueryResponse = layer_climb_proto::ibc::channel::Channel;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::ibc::channel::Channel> {
        let IbcChannelReq {
            channel_id,
            port_id,
            height,
        } = self;

        let mut req = tonic::Request::new(layer_climb_proto::ibc::channel::QueryChannelRequest {
            channel_id: channel_id.to_string(),
            port_id: port_id.to_string(),
        });

        apply_grpc_height(&mut req, *height)?;

        let mut query_client = layer_climb_proto::ibc::channel::query_client::QueryClient::new(
            client.grpc_channel.clone(),
        );

        query_client
            .channel(req)
            .await
            .map(|res| res.into_inner())
            .context("couldn't get channel")?
            .channel
            .context("missing channel")
    }
}

#[derive(Clone, Debug)]
struct IbcCreateClientConsensusStateReq {
    pub trusting_period_secs: Option<u64>,
}

impl QueryRequest for IbcCreateClientConsensusStateReq {
    type QueryResponse = (
        layer_climb_proto::ibc::light_client::ClientState,
        layer_climb_proto::ibc::light_client::ConsensusState,
    );

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<(
        layer_climb_proto::ibc::light_client::ClientState,
        layer_climb_proto::ibc::light_client::ConsensusState,
    )> {
        let trusting_period_secs = self.trusting_period_secs;

        let latest_block_header = BlockHeaderReq { height: None }
            .request(client.clone())
            .await?;

        let consensus_state = layer_climb_proto::ibc::light_client::ConsensusState {
            timestamp: latest_block_header.time(),
            root: Some(layer_climb_proto::MerkleRoot {
                // in MerkleRoot comment itself: "In the Cosmos SDK, the AppHash of a block header becomes the root."
                hash: latest_block_header.app_hash(),
            }),
            next_validators_hash: latest_block_header.next_validators_hash(),
        };

        let staking_params = StakingParamsReq {}.request(client.clone()).await?;

        let unbonding_period = staking_params
            .unbonding_time
            .context("missing unbonding time")?;

        let unbonding_period = layer_climb_proto::Duration {
            seconds: unbonding_period.seconds,
            nanos: unbonding_period.nanos,
        };

        // 2/3 of the unbonding period gives enough time to trust without constant checking
        // but still within enough time to punish misbehaviour
        let trusting_period = match trusting_period_secs {
            Some(trusting_period_secs) => layer_climb_proto::Duration {
                seconds: trusting_period_secs.try_into()?,
                nanos: 0,
            },
            None => layer_climb_proto::Duration {
                seconds: (unbonding_period.seconds * 2) / 3,
                nanos: (unbonding_period.nanos * 2) / 3,
            },
        };

        // value taken from ibc-go tests: https://github.com/cosmos/ibc-go/blob/049bef96f730ee7f29647b1d5833530444395abc/testing/values.go#L33
        let max_clock_drift = layer_climb_proto::Duration {
            seconds: 10,
            nanos: 0,
        };

        let chain_id = client.chain_config.chain_id.to_string();

        let latest_height = layer_climb_proto::RevisionHeight {
            revision_number: client.chain_config.ibc_client_revision()?,
            revision_height: latest_block_header.height()?,
        };

        #[allow(deprecated)]
        let client_state = layer_climb_proto::ibc::light_client::ClientState {
            chain_id,
            // https://github.com/cosmos/ibc-go/blob/049bef96f730ee7f29647b1d5833530444395abc/modules/light-clients/07-tendermint/fraction.go#L9
            // -> https://github.com/cometbft/cometbft/blob/27a460641ad835b9e6ae47523c12b0678b4619a8/light/verifier.go#L15
            trust_level: Some(layer_climb_proto::ibc::light_client::Fraction {
                numerator: 1,
                denominator: 3,
            }),
            trusting_period: Some(trusting_period),
            unbonding_period: Some(unbonding_period),
            max_clock_drift: Some(max_clock_drift),
            frozen_height: None,
            latest_height: Some(latest_height),
            // https://github.com/cosmos/ibc-go/blob/0613ec84a1a38ca797931343f7e2da330ec7c508/modules/core/23-commitment/types/merkle.go#L18
            proof_specs: vec![
                layer_climb_proto::ibc::ics23::iavl_spec(),
                layer_climb_proto::ibc::ics23::tendermint_spec(),
            ],
            // in the ClientState definition itself:
            // > For SDK chains using the default upgrade module, upgrade_path should be []string{"upgrade", "upgradedIBCState"}`
            upgrade_path: vec!["upgrade".to_string(), "upgradedIBCState".to_string()],
            allow_update_after_expiry: false,
            allow_update_after_misbehaviour: false,
        };

        Ok((client_state, consensus_state))
    }
}

#[derive(Debug, Clone)]
pub struct IbcConnectionProofs {
    pub proof_height: layer_climb_proto::RevisionHeight,
    pub consensus_height: layer_climb_proto::RevisionHeight,
    pub query_height: u64,
    pub connection: layer_climb_proto::ibc::connection::ConnectionEnd,
    pub connection_proof: Vec<u8>,
    pub client_state_proof: Vec<u8>,
    pub consensus_proof: Vec<u8>,
    pub client_state: layer_climb_proto::ibc::light_client::ClientState,
}

#[derive(Debug, Clone)]
pub struct IbcChannelProofs {
    pub proof_height: layer_climb_proto::RevisionHeight,
    pub query_height: u64,
    pub channel: layer_climb_proto::ibc::channel::Channel,
    pub channel_proof: Vec<u8>,
}
