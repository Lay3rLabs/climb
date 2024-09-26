// These were used to help debug and develop the client
// ideally they would be solely a backup (like the way abci_proof works)
// some old implementation code is kept in comments for reference

use super::basic::BlockHeightReq;
use crate::prelude::*;

impl QueryClient {
    pub async fn fetch_signed_header(
        &self,
        height: Option<u64>,
    ) -> Result<layer_climb_proto::tendermint::SignedHeader> {
        self.run_with_middleware(SignedHeaderReq { height }).await
    }
    pub async fn fetch_block_events(
        &self,
        block_height: u64,
    ) -> Result<Vec<tendermint::abci::Event>> {
        self.run_with_middleware(BlockEventsReq {
            height: block_height,
        })
        .await
    }
}

#[derive(Clone, Debug)]
struct SignedHeaderReq {
    pub height: Option<u64>,
}

impl QueryRequest for SignedHeaderReq {
    type QueryResponse = layer_climb_proto::tendermint::SignedHeader;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::tendermint::SignedHeader> {
        let height = match self.height {
            Some(height) => height,
            None => BlockHeightReq {}.request(client.clone()).await?,
        };

        Ok(client.rpc_client.commit(height).await?.signed_header.into())
    }
}

#[derive(Clone, Debug)]
struct BlockEventsReq {
    pub height: u64,
}

impl QueryRequest for BlockEventsReq {
    type QueryResponse = Vec<tendermint::abci::Event>;

    async fn request(&self, client: QueryClient) -> Result<Vec<tendermint::abci::Event>> {
        let mut response = client.rpc_client.block_results(self.height).await?;

        let mut events: Vec<tendermint::abci::Event> = vec![];

        if let Some(begin_block_events) = &mut response.begin_block_events {
            events.append(begin_block_events);
        }

        if let Some(txs_results) = &mut response.txs_results {
            for tx_result in txs_results {
                if tx_result.code != tendermint::abci::Code::Ok {
                    // Transaction failed, skip it
                    continue;
                }

                events.append(&mut tx_result.events);
            }
        }

        if let Some(end_block_events) = &mut response.end_block_events {
            events.append(end_block_events);
        }

        Ok(events)
    }
}

// yes, this is ridiculous
// pub fn convert_rpc_signed_header(signed_header: layer_climb_proto::SignedHeader) -> layer_climb_proto::SignedHeader {
//     layer_climb_proto::SignedHeader {
//         header: signed_header.header.map(|header| {
//             layer_climb_proto::Header {
//                 version: header.version.map(|consensus| {
//                     tendermint_layer_climb_proto::version::Consensus {
//                         block: consensus.block,
//                         app: consensus.app,
//                     }
//                 }),
//                 chain_id: header.chain_id,
//                 height: header.height,
//                 time: header.time.map(|time| {
//                     layer_climb_proto::Timestamp {
//                         seconds: time.seconds,
//                         nanos: time.nanos,
//                     }
//                 }),
//                 last_block_id: header.last_block_id.map(|block_id| {
//                     layer_climb_proto::BlockId {
//                         hash: block_id.hash,
//                         part_set_header: block_id.part_set_header.map(|part_set_header| {
//                             layer_climb_proto::PartSetHeader {
//                                 total: part_set_header.total,
//                                 hash: part_set_header.hash,
//                             }
//                         }),
//                     }
//                 }),
//                 last_commit_hash: header.last_commit_hash,
//                 data_hash: header.data_hash,
//                 validators_hash: header.validators_hash,
//                 next_validators_hash: header.next_validators_hash,
//                 consensus_hash: header.consensus_hash,
//                 app_hash: header.app_hash,
//                 last_results_hash: header.last_results_hash,
//                 evidence_hash: header.evidence_hash,
//                 proposer_address: header.proposer_address
//             }
//         }),

//         commit: signed_header.commit.map(|commit| {
//             layer_climb_proto::Commit {
//                 height: commit.height,
//                 round: commit.round,
//                 block_id: commit.block_id.map(|block_id| {
//                     layer_climb_proto::BlockId {
//                         hash: block_id.hash,
//                         part_set_header: block_id.part_set_header.map(|part_set_header| {
//                             layer_climb_proto::PartSetHeader {
//                                 total: part_set_header.total,
//                                 hash: part_set_header.hash,
//                             }
//                         }),
//                     }
//                 }),
//                 signatures: commit.signatures.into_iter().map(|signature| {
//                     layer_climb_proto::CommitSig {
//                         block_id_flag: signature.block_id_flag as i32,
//                         validator_address: signature.validator_address,
//                         timestamp: signature.timestamp.map(|timestamp| {
//                             layer_climb_proto::Timestamp {
//                                 seconds: timestamp.seconds,
//                                 nanos: timestamp.nanos,
//                             }
//                         }),
//                         signature: signature.signature,
//                     }
//                 }).collect(),
//             }
//         })
//     }

// }

// #[derive(Debug, Clone)]
// pub struct IbcClientStateWithProof{
//     pub client_state: layer_climb_proto::ibc::light_client::ClientState,
//     pub proof: Vec<u8>,
// }

// #[derive(Debug, Clone)]
// pub struct IbcConnectionWithProof {
//     pub connection: layer_climb_proto::ibc::connection::ConnectionEnd,
//     pub proof: Vec<u8>,
// }

// #[derive(Debug, Clone)]
// pub struct IbcConsensusStateWithProof {
//     pub consensus_state: Vec<u8>,
//     pub proof: Vec<u8>,
// }

// impl QueryClient {

//     async fn fetch_ibc_connection_proofs(&self, proof_height: layer_climb_proto::RevisionHeight, client_id: &IbcClientId, connection_id: &IbcConnectionId) -> Result<IbcConnectionProofs> {
//         let query_height = proof_height.revision_height - 1;

//         // all-in-one RPC style:
//         let IbcConnectionWithProof{connection, proof: connection_proof}
//             = self.fetch_ibc_connection_with_proof(connection_id, query_height).await?;

//         let IbcClientStateWithProof{client_state, proof: client_state_proof}
//             = self.fetch_ibc_client_state_with_proof(client_id, query_height).await?;

//         let consensus_height = client_state.latest_height.as_ref().context("missing client state latest height")?.clone();

//         let IbcConsensusStateWithProof {proof: consensus_proof, ..} =
//             self.fetch_ibc_consensus_state_with_proof(client_id, consensus_height, query_height).await?;

//         if client_state_proof.is_empty() {
//             bail!("missing client state proof");
//         }
//         if connection_proof.is_empty() {
//             bail!("missing connection proof");
//         }
//         if consensus_proof.is_empty() {
//             bail!("missing consensus proof");
//         }

//         Ok(IbcConnectionProofs {
//             proof_height,
//             consensus_height,
//             query_height,
//             connection,
//             connection_proof,
//             client_state_proof,
//             consensus_proof,
//             client_state,
//         })
//     }
//     async fn fetch_ibc_client_state_with_proof(&self, ibc_client_id: &IbcClientId, height: u64) -> Result<IbcClientStateWithProof> {
//         let resp = self.fetch_ibc_abci_query(IbcAbciProof::ClientState { client_id: ibc_client_id.clone() }, height, true).await?;
//         let client_state = layer_climb_proto::Any::decode(resp.value.as_slice())?;
//         let client_state = match client_state.type_url.as_str() {
//             "/ibc.lightclients.tendermint.v1.ClientState" => {
//                 layer_climb_proto::ibc::light_client::ClientState::decode(client_state.value.as_slice())
//                     .map_err(|e| e.into())
//             },
//             _ => Err(anyhow::anyhow!("unsupported client state type: {}", client_state.type_url)),
//         }?;

//         let proof = resp.proof.context("missing proof")?;
//         let proof = convert_rpc_proof(proof)?;

//         Ok(IbcClientStateWithProof{ client_state, proof})
//     }

//     async fn fetch_ibc_connection_with_proof(&self, connection_id: &IbcConnectionId, height: u64) -> Result<IbcConnectionWithProof> {
//         let resp = self.fetch_ibc_abci_query(IbcAbciProof::Connection { connection_id: connection_id.clone() }, height, true).await?;
//         let connection = layer_climb_proto::ibc::connection::ConnectionEnd::decode(resp.value.as_slice())?;
//         let proof = resp.proof.context("missing proof")?;
//         let proof = convert_rpc_proof(proof)?;

//         Ok(IbcConnectionWithProof { connection, proof})
//     }

//     async fn fetch_ibc_consensus_state_with_proof(&self, client_id: &IbcClientId, consensus_height: layer_climb_proto::RevisionHeight, height: u64) -> Result<IbcConsensusStateWithProof> {
//         let resp = self.fetch_ibc_abci_query(IbcAbciProof::Consensus { client_id: client_id.clone(), height: consensus_height.clone() }, height, true).await?;
//         let proof = resp.proof.context("missing proof")?;
//         let proof = convert_rpc_proof(proof)?;

//         Ok(IbcConsensusStateWithProof { consensus_state: resp.value, proof})
//     }

//     async fn fetch_ibc_validator_set(&self, height: Option<u64>, proposer_address: Option<tendermint::account::Id>) -> Result<layer_climb_proto::tendermint::types::ValidatorSet> {
//         let height = match height {
//             Some(height) => height,
//             None => self.query_block_height().await?,
//         };

//         let height = tendermint::block::Height::try_from(height)?;

//         let validators = self.http_client().rpc.validators(height, tendermint_rpc::Paging::All).await?.validators;

//         let proposer_address = proposer_address.and_then(|proposer_address| validators.iter().find(|validator| validator.address == proposer_address).cloned());

//         Ok(convert_raw_validator_set(tendermint::validator::Set::new(validators, proposer_address)))
//     }

// }

// pub(super) fn convert_rpc_proof(proof: tendermint::merkle::proof::ProofOps) -> Result<Vec<u8>> {
//     let mut proofs = Vec::new();

//     for op in &proof.ops {
//         let mut parsed = layer_climb_proto::ibc::ics23::CommitmentProof { proof: None };
//         layer_climb_proto::prost::Message::merge(&mut parsed, op.data.as_slice())?;
//         // if let layer_climb_proto::ibc::ics23::commitment_proof::Proof::Exist(layer_climb_proto::ibc::ics23::ExistenceProof{ key, value, leaf, path}) = parsed.proof.as_mut().unwrap() {
//         //     println!("{} vs. {:?}", op.field_type, path);
//         // }
//         proofs.push(parsed);
//     }

//     let merkle_proof = layer_climb_proto::ibc::ibc::core::commitment::v1::MerkleProof { proofs };

//     let mut bytes = Vec::new();
//     layer_climb_proto::prost::Message::encode(&merkle_proof, &mut bytes)?;

//     Ok(bytes)
// }
