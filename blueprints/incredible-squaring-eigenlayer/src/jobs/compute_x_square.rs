#![allow(dead_code)]
use crate::contexts::client::{AggregatorClient, SignedTaskResponse};
use crate::{noop, IncredibleSquaringTaskManager, INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING};
use alloy_contract::ContractInstance;
use alloy_network::Ethereum;
use alloy_primitives::{keccak256, Address};
use alloy_primitives::{hex, Bytes, FixedBytes, U256};
use alloy_sol_types::private::alloy_json_abi::JsonAbi;
use alloy_sol_types::SolType;
use ark_bn254::Fq;
use ark_ff::{BigInteger, PrimeField};
use color_eyre::Result;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::crypto_bls::OperatorId;
use gadget_sdk::{error, info, job};
use std::{convert::Infallible, ops::Deref, sync::OnceLock};
use gadget_sdk::event_listener::evm_contracts::EthereumContractBound;
use IncredibleSquaringTaskManager::TaskResponse;
use crate::IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance;

#[doc = "Job definition for the function "]
#[doc = "[`"]
#[doc = "xsquare_eigen"]
#[doc = "`]"]
#[automatically_derived]
#[doc(hidden)]
pub const XSQUARE_EIGEN_JOB_DEF: &str = "{\"metadata\":{\"name\":\"xsquare_eigen\",\"description\":null},\"params\":[\"U256\",\"Uint32\",\"Bytes\",\"Uint8\",\"Uint32\"],\"result\":[\"Uint32\"]}";
#[doc = "Job ID for the function "]
#[doc = "[`"]
#[doc = "xsquare_eigen"]
#[doc = "`]"]
#[automatically_derived]
pub const XSQUARE_EIGEN_JOB_ID: u8 = 0;
static XSQUARE_EIGEN_ACTIVE_CALL_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[doc = r" Event handler for the function"]
#[doc = "[`"]
#[doc = "xsquare_eigen"]
#[doc = "`]"]
#[derive(Clone)]
pub struct XsquareEigenEventHandler<T: EthereumContractBound> {

    pub contract: IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance<T::TH, T::PH>,
    pub contract_instance: OnceLock<ContractInstance<T::TH, T::PH, Ethereum>>,
    pub ctx: AggregatorClient,
}

#[gadget_sdk::async_trait::async_trait]
impl<T: EthereumContractBound> gadget_sdk::events_watcher::InitializableEventHandler for XsquareEigenEventHandler<T> {
    async fn init_event_handler(&self ) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>> {
        let mut listeners = vec![];
        listeners.push(run_listener_xsquare_eigen_0eventhandler(&self).await.expect("Event listener already initialized"));
        let (tx, rx) = gadget_sdk::tokio::sync::oneshot::channel();
        let task = async move {
            let mut futures = gadget_sdk::futures::stream::FuturesUnordered::new();
            for listener in listeners { futures.push(listener); }
            if let Some(res) = gadget_sdk::futures::stream::StreamExt::next(&mut futures).await {
                gadget_sdk::error!("An Event Handler for {} has stopped running" , stringify ! (XsquareEigenEventHandler ));
                let res = match res {
                    Ok(res) => { res }
                    Err(e) => { Err(gadget_sdk::Error::Other(format!("Error in Event Handler for {}: {e:?}", stringify!(XsquareEigenEventHandler )))) }
                };
                tx.send(res).unwrap();
            }
        };
        let _ = gadget_sdk::tokio::spawn(task);
        Some(rx)
    }
}
async fn run_listener_xsquare_eigen_0eventhandler<T: gadget_sdk::event_listener::evm_contracts::EthereumContractBound>(ctx: &XsquareEigenEventHandler<T>) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>> {
    static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !ONCE.load(std::sync::atomic::Ordering::Relaxed) {
        ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
        let (tx, rx) = gadget_sdk::tokio::sync::oneshot::channel();
        let ctx = ctx.clone();
        let mut instance = <gadget_sdk::event_listener::evm_contracts::EthereumHandlerWrapper<_, _,  _> as gadget_sdk::event_listener::EventListener::<_, _>>::new(&ctx).await.expect("Failed to create event listener");
        let task = async move {
            let res = gadget_sdk::event_listener::EventListener::<_, _>::execute(&mut instance).await;
            let _ = tx.send(res);
        };
        gadget_sdk::tokio::task::spawn(task);
        return Some(rx);
    }
    None
}

impl<T: gadget_sdk::event_listener::evm_contracts::EthereumContractBound> gadget_sdk::event_listener::evm_contracts::EvmContractInstance<T> for XsquareEigenEventHandler<T> {
    fn get_instance(&self) -> &ContractInstance<T::TH, T::PH, Ethereum> {
        self.get_contract_instance()
    }
}

#[allow(unused_variables)]
#[doc = " Returns x^2 saturating to [`u64::MAX`] if overflow occurs."]

pub async fn xsquare_eigen(
    ctx: AggregatorClient,
    number_to_be_squared: U256,
    task_created_block: u32,
    quorum_numbers: Bytes,
    quorum_threshold_percentage: u8,
    task_index: u32,
) -> Result<u32, Infallible> {
    // Calculate our response to job
    let task_response = TaskResponse {
        referenceTaskIndex: task_index,
        numberSquared: number_to_be_squared.saturating_pow(U256::from(2u32)),
    };

    let bls_key_pair = BlsKeyPair::new(
        "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
            .to_string(),
    )
        .unwrap();

    let operator_id = alloy_primitives::FixedBytes(
        eigensdk::types::operator::operator_id_from_g1_pub_key(bls_key_pair.public_key()).unwrap(),
    );
    let operator_id: OperatorId =
        hex!("fd329fe7e54f459b9c104064efe0172db113a50b5f394949b4ef80b3c34ca7f5").into();

    // Sign the Hashed Message and send it to the BLS Aggregator
    let msg_hash = keccak256(<TaskResponse as SolType>::abi_encode(&task_response));
    let signed_response = SignedTaskResponse {
        task_response,
        signature: bls_key_pair.sign_message(msg_hash.as_ref()),
        operator_id,
    };

    info!(
        "Sending signed task response to BLS Aggregator: {:#?}",
        signed_response
    );
    if let Err(e) = ctx.send_signed_task_response(signed_response).await {
        error!("Failed to send signed task response: {:?}", e);
        return Ok(0);
    }

    Ok(1)
}

impl<T: EthereumContractBound> XsquareEigenEventHandler<T>
{
    #[doc = r" Lazily creates the [`ContractInstance`] if it does not exist, otherwise returning a reference to it."]
    #[allow(clippy::clone_on_copy)]
    fn get_contract_instance(&self) -> &ContractInstance<T::TH, T::PH, Ethereum> {
        self.contract_instance.get_or_init(|| {
            let abi_location = alloy_contract::Interface::new(JsonAbi::from_json_str(&INCREDIBLE_SQUARING_TASK_MANAGER_ABI_STRING).unwrap());
            ContractInstance::new(self.contract.address().clone(), self.contract.provider().clone(), abi_location )
        })
    }
}
impl<T: EthereumContractBound> Deref for XsquareEigenEventHandler<T>
{
    type Target = ContractInstance<T::TH, T::PH, Ethereum>;
    fn deref(&self) -> &Self::Target { self.get_contract_instance() }
}
#[automatically_derived]
#[gadget_sdk::async_trait::async_trait]
impl<T: EthereumContractBound> gadget_sdk::events_watcher::evm::EvmEventHandler<T> for XsquareEigenEventHandler<T>
{
    type Event = IncredibleSquaringTaskManager::NewTaskCreated;
    const GENESIS_TX_HASH: FixedBytes<32> = FixedBytes([0; 32]);
    async fn handle(&self, log: &gadget_sdk::alloy_rpc_types::Log, event: &Self::Event) -> Result<(), gadget_sdk::events_watcher::Error> {
        use alloy_provider::Provider;
        use alloy_sol_types::SolEvent;
        use alloy_sol_types::SolInterface;
        let contract = &self.contract;
        let decoded: alloy_primitives::Log<Self::Event> = <Self::Event as SolEvent>::decode_log(&log.inner, true)?;
        let (_, index) = decoded.topics();
        let inputs = convert_event_to_inputs(decoded.data, index);
        let param0 = inputs.0;
        let param1 = inputs.1;
        let param2 = inputs.2;
        let param3 = inputs.3;
        let param4 = inputs.4;
        let job_result = match xsquare_eigen(
            self.ctx.clone(), param0, param1, param2, param3, param4, ).await {
            Ok(r) => r,
            Err(e) => {
                ::gadget_sdk::error!("Error in job: {e}" );
                let error = gadget_sdk::events_watcher::Error::Handler(Box::new(e));
                return Err(error);
            }
        };   ;
        let call = noop(job_result);
        Ok(())
    }
}
impl<T: EthereumContractBound> gadget_sdk::event_listener::markers::IsEvm for XsquareEigenEventHandler<T> {}

/// Converts the event to inputs.
///
/// Uses a tuple to represent the return type because
/// the macro will index all values in the #[job] function
/// and parse the return type by the index.
pub fn convert_event_to_inputs(
    event: IncredibleSquaringTaskManager::NewTaskCreated,
    _index: u32,
) -> (U256, u32, Bytes, u8, u32) {
    let task_index = event.taskIndex;
    let number_to_be_squared = event.task.numberToBeSquared;
    let task_created_block = event.task.taskCreatedBlock;
    let quorum_numbers = event.task.quorumNumbers;
    let quorum_threshold_percentage = event.task.quorumThresholdPercentage.try_into().unwrap();
    (
        number_to_be_squared,
        task_created_block,
        quorum_numbers,
        quorum_threshold_percentage,
        task_index,
    )
}

/// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
/// (U256 reads data as big endian)
pub fn point_to_u256(point: Fq) -> U256 {
    let point = point.into_bigint();
    let point_bytes = point.to_bytes_be();
    U256::from_be_slice(&point_bytes[..])
}
