use crate::client_ext::job_types::{JobCircuitProperties, JobProperties};
use async_trait::async_trait;
use gadget_core::gadget::substrate::Client;
use sp_runtime::traits::Block;

#[async_trait]
pub trait ClientWithApi<B: Block>: Client<B> + Clone {
    async fn get_job_circuit_properties(
        &self,
        job_id: u64,
    ) -> Result<JobCircuitProperties, webb_gadget::Error>;
    async fn get_job_properties(&self, job_id: u64) -> Result<JobProperties, webb_gadget::Error>;
}

pub mod job_types {
    pub struct ZkJob {
        pub circuit: JobCircuitProperties,
        pub properties: JobProperties,
    }

    pub struct JobCircuitProperties {
        pub job_id: u64,
        pub pk: Vec<u8>,
        pub wasm_uri: String,
        pub r1cs_uri: String,
    }

    pub struct JobProperties {
        pub job_id: u64,
        pub circuit_id: u64,
        pub public_inputs: Vec<u8>,
        pub pss: u64,
        pub a_shares: Vec<u8>,
        pub ax_shares: Vec<u8>,
        pub qap_shares: Vec<u8>,
    }
}
