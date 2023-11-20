use async_trait::async_trait;
use gadget_core::gadget::substrate::Client;
use sp_runtime::traits::Block;

use self::job_types::{CircuitProperties, JobProperties};

#[async_trait]
pub trait ClientWithApi<B: Block>: Client<B> + Clone {
    async fn get_job_circuit_properties(
        &self,
        circuit_id: u64,
    ) -> Result<Option<CircuitProperties>, webb_gadget::Error>;

    async fn get_job_properties(
        &self,
        job_id: u64,
    ) -> Result<Option<JobProperties>, webb_gadget::Error>;

    async fn get_next_job(&self) -> Result<Option<JobProperties>, webb_gadget::Error>;
}

pub mod job_types {
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ZkJob {
        pub circuit: CircuitProperties,
        pub properties: JobProperties,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CircuitProperties {
        pub circuit_id: u64,
        pub pk_uri: PathBuf,
        pub wasm_uri: PathBuf,
        pub r1cs_uri: PathBuf,
        pub num_inputs: usize,
        pub num_constraints: usize,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PackedQAPShare {
        pub a: Vec<ByteBuf>,
        pub b: Vec<ByteBuf>,
        pub c: Vec<ByteBuf>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct JobProperties {
        pub job_id: u64,
        pub circuit_id: u64,
        pub public_inputs: Vec<String>,
        pub pss_l: usize,
        pub a_shares: Vec<Vec<ByteBuf>>,
        pub ax_shares: Vec<Vec<ByteBuf>>,
        pub qap_shares: Vec<PackedQAPShare>,
    }
}
