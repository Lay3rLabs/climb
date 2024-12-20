use tracing::instrument;

use crate::{
    ibc_types::{IbcChannelId, IbcClientId, IbcConnectionId, IbcPortId},
    prelude::*,
};

use super::ConnectionMode;

impl QueryClient {
    // from looking at other implementations, it might seem like getting proof_height from the current remote block height is the way to go
    // ... but, it just doesn't seem to work.
    // instead, getting proof height from the client state (i.e. local client state, which is the state of the remote chain) seems to work just fine

    // height - 1 is documented here: https://github.com/cosmos/ibc-go/blob/main/modules/core/client/query.go#L26
    #[instrument]
    pub async fn abci_proof(&self, kind: AbciProofKind, height: u64) -> Result<AbciProofResponse> {
        self.run_with_middleware(AbciProofReq {
            kind: kind.clone(),
            height,
        })
        .await
    }
}

#[derive(Clone, Debug)]
pub struct AbciProofReq {
    pub kind: AbciProofKind,
    pub height: u64,
}

#[derive(Clone, Debug)]
pub enum AbciProofKind {
    IbcClientState {
        client_id: IbcClientId,
    },
    IbcConnection {
        connection_id: IbcConnectionId,
    },
    IbcConsensus {
        client_id: IbcClientId,
        height: layer_climb_proto::RevisionHeight,
    },
    IbcChannel {
        channel_id: IbcChannelId,
        port_id: IbcPortId,
    },
    IbcPacketCommitment {
        port_id: IbcPortId,
        channel_id: IbcChannelId,
        sequence: u64,
    },
    IbcPacketReceive {
        port_id: IbcPortId,
        channel_id: IbcChannelId,
        sequence: u64,
    },
    IbcPacketAck {
        port_id: IbcPortId,
        channel_id: IbcChannelId,
        sequence: u64,
    },
    StakingParams,
    AuthBaseAccount {
        address: Address,
    },
}
impl AbciProofKind {
    pub fn path(&self) -> &str {
        match self {
            Self::IbcClientState { .. }
            | Self::IbcConnection { .. }
            | Self::IbcConsensus { .. }
            | Self::IbcChannel { .. }
            | Self::IbcPacketCommitment { .. }
            | Self::IbcPacketReceive { .. }
            | Self::IbcPacketAck { .. } => "store/ibc/key",
            Self::StakingParams => "store/staking/key",
            Self::AuthBaseAccount { .. } => "store/acc/key",
        }
    }
    pub fn data_bytes(&self) -> Vec<u8> {
        // https://github.com/cosmos/ibc/blob/main/spec/core/ics-024-host-requirements/README.md
        match self {
            Self::IbcClientState { client_id } => {
                format!("clients/{client_id}/clientState").into_bytes()
            }
            Self::IbcConnection { connection_id } => {
                format!("connections/{connection_id}").into_bytes()
            }
            Self::IbcConsensus { client_id, height } => format!(
                "clients/{client_id}/consensusStates/{}-{}",
                height.revision_number, height.revision_height
            )
            .into_bytes(),
            Self::IbcChannel {
                channel_id,
                port_id,
            } => format!("channelEnds/ports/{port_id}/channels/{channel_id}").into_bytes(),
            Self::IbcPacketCommitment {
                port_id,
                channel_id,
                sequence,
            } => format!("commitments/ports/{port_id}/channels/{channel_id}/sequences/{sequence}")
                .into_bytes(),
            Self::IbcPacketReceive {
                port_id,
                channel_id,
                sequence,
            } => format!("receipts/ports/{port_id}/channels/{channel_id}/sequences/{sequence}")
                .into_bytes(),
            Self::IbcPacketAck {
                port_id,
                channel_id,
                sequence,
            } => format!("acks/ports/{port_id}/channels/{channel_id}/sequences/{sequence}")
                .into_bytes(),
            Self::StakingParams => vec![0x01],
            Self::AuthBaseAccount { address } => {
                let mut data = vec![0x01];
                data.extend(address.as_bytes());
                data
            }
        }
    }
}

#[derive(Debug)]
pub struct AbciProofResponse {
    pub proof: Vec<u8>,
    pub value: Vec<u8>,
    // could add more fields like height, but imho that's more confusing than helpful for now
}

impl QueryRequest for AbciProofReq {
    type QueryResponse = AbciProofResponse;

    async fn request(&self, client: QueryClient) -> Result<AbciProofResponse> {
        match client.get_connection_mode() {
            ConnectionMode::Grpc => {
                let req = tonic::Request::new(layer_climb_proto::tendermint::AbciQueryRequest {
                    path: self.kind.path().to_string(),
                    data: self.kind.data_bytes(),
                    height: self.height.try_into()?,
                    prove: true,
                });

                // I think, don't do this, since height is part of the request?
                // apply_grpc_height(&mut req, Some(self.height))?;

                let mut query_client =
                    layer_climb_proto::tendermint::service_client::ServiceClient::new(
                        client.grpc_channel.clone(),
                    );
                let resp: layer_climb_proto::tendermint::AbciQueryResponse = query_client
                    .abci_query(req)
                    .await
                    .map(|res| res.into_inner())
                    .with_context(|| format!("couldn't get abci proof for {:?}", self.kind))?;

                //log_abci_resp(&resp, &self.kind, self.height);

                // get a byte-string from resp.value which is a Vec<u8>:
                let proof_ops = resp.proof_ops.context("missing proof_ops in abci_query")?;

                let proof = AbciProofToConvert::Grpc(proof_ops).convert_abci_proof()?;

                Ok(AbciProofResponse {
                    proof,
                    value: resp.value,
                })
            }
            ConnectionMode::Rpc => {
                // https://github.com/cosmos/ibc-go/blob/73061ee020a6be676f2d5843b7430082d2fe275c/modules/core/client/query.go#L26
                let resp = client
                    .rpc_client
                    .abci_query(
                        self.kind.path().to_string(),
                        self.kind.data_bytes(),
                        self.height,
                        true,
                    )
                    .await?;

                let proof_ops = resp.proof.context("missing proof_ops in abci_query")?;

                let proof = AbciProofToConvert::Rpc(proof_ops).convert_abci_proof()?;

                Ok(AbciProofResponse {
                    proof,
                    value: resp.value,
                })
            }
        }
    }
}

enum AbciProofToConvert {
    Grpc(layer_climb_proto::tendermint::ProofOps),
    Rpc(tendermint::merkle::proof::ProofOps),
}

impl AbciProofToConvert {
    fn into_vec(self) -> Vec<Vec<u8>> {
        match self {
            Self::Grpc(proof_ops) => proof_ops.ops.into_iter().map(|op| op.data).collect(),
            Self::Rpc(proof_ops) => proof_ops.ops.into_iter().map(|op| op.data).collect(),
        }
    }

    fn convert_abci_proof(self) -> Result<Vec<u8>> {
        let mut proofs = Vec::new();

        for op in self.into_vec() {
            let mut parsed = layer_climb_proto::ibc::ics23::CommitmentProof { proof: None };
            layer_climb_proto::Message::merge(&mut parsed, op.as_slice())?;
            // if let layer_climb_proto::ibc::ics23::commitment_proof::Proof::Exist(layer_climb_proto::ibc::ics23::ExistenceProof{ key, value, leaf, path}) = parsed.proof.as_mut().unwrap() {
            //     println!("{} vs. {:?}", op.field_type, path);
            // }
            proofs.push(parsed);
        }

        let merkle_proof = layer_climb_proto::MerkleProof { proofs };

        let mut bytes = Vec::new();
        layer_climb_proto::Message::encode(&merkle_proof, &mut bytes)?;

        Ok(bytes)
    }
}

// in theory, with this working, we could save some requests... but, it's finicky
// fn log_abci_resp(resp: &layer_climb_proto::cosmos::base::tendermint::v1beta1::AbciQueryResponse, kind: &AbciProofKind, height: u64 ) {
//     match layer_climb_proto::Any::decode(resp.value.as_slice()) {
//         Ok(any) => {
//             match any.type_url.as_str() {
//                 "/ibc.lightclients.tendermint.v1.ClientState" => {
//                     match layer_climb_proto::ibc::light_client::ClientState::decode(any.value.as_slice()) {
//                         Ok(client_state) => {
//                             println!("abci_query response for {} at height {}: {:?}", kind.data_string(), height, client_state);
//                         },
//                         Err(e) => {
//                             println!("abci_query response for {} at height {}: [error decoding client state] {}", kind.data_string(), height, e);
//                         }
//                     }
//                 },
//                 "/ibc.lightclients.tendermint.v1.ConsensusState" => {
//                     match layer_climb_proto::ibc::light_client::ConsensusState::decode(any.value.as_slice()) {
//                         Ok(consensus_state) => {
//                             println!("abci_query response for {} at height {}: {:?}", kind.data_string(), height, consensus_state);
//                         },
//                         Err(e) => {
//                             println!("abci_query response for {} at height {}: [error decoding consensus state] {}", kind.data_string(), height, e);
//                         }
//                     }
//                 },
//                 _ => {
//                     println!("abci_query response for {} at height {}: [unknown protobuf] {}", kind.data_string(), height, any.type_url);
//                 }
//             }
//         },
//         Err(_) => {
//             println!("abci_query response for {} at height {}: {}", kind.data_string(), height, std::str::from_utf8(&resp.value).unwrap_or("[COULDN'T DECODE VALUE]"));
//         }
//     }
// }
