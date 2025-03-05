use alloy_primitives::Address;
use gadget_utils_evm::get_provider_http;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Contract error: {0}")]
    Contract(#[from] alloy_contract::Error),
}

/// Get the allocation manager address from the `DelegationManager` contract
///
/// # Returns
/// - [`Address`] - The allocation manager address
///
/// # Errors
/// - [`Error::AlloyContract`] - If the call to the contract fails (i.e. the contract doesn't exist at the given address)
pub async fn get_allocation_manager_address(
    delegation_manager_addr: Address,
    http_endpoint: &str,
) -> Result<Address, Error> {
    let provider = get_provider_http(http_endpoint);
    let delegation_manager =
        eigensdk::utils::slashing::core::delegationmanager::DelegationManager::DelegationManagerInstance::new(
            delegation_manager_addr,
            provider,
        );
    delegation_manager
        .allocationManager()
        .call()
        .await
        .map(|a| a._0)
        .map_err(Error::Contract)
}
