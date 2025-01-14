use tracing::instrument;

use crate::prelude::*;

use super::basic::BlockHeightReq;

impl QueryClient {
    #[instrument]
    pub async fn validator_set(
        &self,
        height: Option<u64>,
        proposer_address: Option<&[u8]>,
    ) -> Result<layer_climb_proto::tendermint::ValidatorSet> {
        self.run_with_middleware(ValidatorSetReq {
            height,
            proposer_address,
        })
        .await
    }
}

#[derive(Clone, Debug)]
pub struct ValidatorSetReq<'a> {
    pub height: Option<u64>,
    pub proposer_address: Option<&'a [u8]>,
}

impl QueryRequest for ValidatorSetReq<'_> {
    type QueryResponse = layer_climb_proto::tendermint::ValidatorSet;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::tendermint::ValidatorSet> {
        let height = match self.height {
            Some(height) => height,
            None => BlockHeightReq {}.request(client.clone()).await?,
        };

        let mut validators = Vec::new();
        let mut pagination = None;

        let mut proposer = None;

        let proposer_address = self
            .proposer_address
            .map(|addr| tendermint::account::Id::try_from(addr.to_vec()))
            .transpose()?;

        let mut grpc_query_client = match client.get_connection_mode() {
            ConnectionMode::Grpc => Some(
                layer_climb_proto::tendermint::service_client::ServiceClient::new(
                    client.clone_grpc_channel()?,
                ),
            ),
            ConnectionMode::Rpc => None,
        };

        loop {
            let req = layer_climb_proto::tendermint::GetValidatorSetByHeightRequest {
                height: height.try_into()?,
                pagination,
            };

            let resp = match client.get_connection_mode() {
                ConnectionMode::Grpc => grpc_query_client
                    .as_mut()
                    .unwrap()
                    .get_validator_set_by_height(req)
                    .await
                    .map(|res| res.into_inner())
                    .context("couldn't get validator set")?,
                ConnectionMode::Rpc => client
                    .rpc_client()?
                    .abci_protobuf_query(
                        "/cosmos.base.tendermint.v1beta1.Service/GetValidatorSetByHeight",
                        req,
                        Some(height),
                    )
                    .await
                    .context("couldn't get validator set")?,
            };

            for validator in resp.validators {
                let pub_key = validator.pub_key.context("couldn't get public key")?;

                let pub_key = match pub_key.type_url.as_str() {
                    "/cosmos.crypto.ed25519.PubKey" => {
                        let key = layer_climb_proto::crypto::ed25519::PubKey::decode(
                            pub_key.value.as_slice(),
                        )?
                        .key;
                        tendermint::public_key::PublicKey::Ed25519(key.as_slice().try_into()?)
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "unsupported public key type: {}",
                            pub_key.type_url
                        ))
                    }
                };

                let info = tendermint::validator::Info {
                    name: None,
                    pub_key,
                    address: tendermint::account::Id::from(pub_key),
                    power: validator.voting_power.try_into()?,
                    proposer_priority: validator.proposer_priority.into(),
                };

                if Some(info.address) == proposer_address {
                    proposer = Some(info.clone());
                }

                validators.push(info);
            }

            if let Some(resp_pagination) = resp.pagination {
                if resp_pagination.next_key.is_empty() {
                    break;
                } else {
                    pagination = Some(layer_climb_proto::query::PageRequest {
                        key: resp_pagination.next_key,
                        offset: 0,
                        limit: 0,
                        count_total: false,
                        reverse: false,
                    });
                }
            } else {
                break;
            }
        }

        let validator_set =
            convert_raw_validator_set(tendermint::validator::Set::new(validators, proposer));

        // validators.sort_by_key(|v| (core::cmp::Reverse(v.voting_power), v.address.clone()));
        // let validator_set = layer_climb_proto::ValidatorSet {
        //     validators,
        //     proposer,
        //     total_voting_power,
        // };

        Ok(validator_set)
    }
}

// yes, this is ridiculous
fn convert_raw_validator_set(
    validators: tendermint::validator::Set,
) -> layer_climb_proto::tendermint::ValidatorSet {
    layer_climb_proto::tendermint::ValidatorSet {
        validators: validators
            .validators()
            .iter()
            .map(|validator| layer_climb_proto::tendermint::Validator {
                address: validator.address.into(),
                pub_key: Some(layer_climb_proto::tendermint::crypto::PublicKey {
                    sum: Some(
                        layer_climb_proto::tendermint::crypto::public_key::Sum::Ed25519(
                            validator.pub_key.to_bytes(),
                        ),
                    ),
                }),
                voting_power: validator.power.into(),
                proposer_priority: validator.proposer_priority.into(),
            })
            .collect(),

        proposer: validators.proposer().as_ref().map(|validator| {
            layer_climb_proto::tendermint::Validator {
                address: validator.address.into(),
                pub_key: Some(layer_climb_proto::tendermint::crypto::PublicKey {
                    sum: Some(
                        layer_climb_proto::tendermint::crypto::public_key::Sum::Ed25519(
                            validator.pub_key.to_bytes(),
                        ),
                    ),
                }),
                voting_power: validator.power.into(),
                proposer_priority: validator.proposer_priority.into(),
            }
        }),
        total_voting_power: validators.total_voting_power().into(),
    }
}
