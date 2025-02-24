//! Transaction extractors for EVM
//!
//! Extractors for transaction-related data from EVM blocks.

use alloy_primitives::{Address, Bytes, B256};
use alloy_rpc_types::{Block, Transaction};
use blueprint_job_router::{
    __define_rejection as define_rejection, __impl_deref as impl_deref,
    __impl_deref_vec as impl_deref_vec, job_call::Parts as JobCallParts, FromJobCallParts,
};

/// Extracts all transactions in a block that target a specific contract address
#[derive(Debug, Clone)]
pub struct ContractTransactions {
    /// The target contract address
    pub contract: Address,
    /// The matching transactions
    pub transactions: Vec<Transaction>,
}

impl_deref_vec!(ContractTransactions: Transaction);

define_rejection! {
    #[body = "No block found in extensions"]
    /// This rejection is used to indicate that a block was not found in the extensions.
    pub struct MissingBlock;
}

impl TryFrom<&mut JobCallParts> for ContractTransactions {
    type Error = MissingBlock;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block = parts
            .extensions
            .get::<Block<Transaction>>()
            .ok_or(MissingBlock)?;

        // Get the contract address from extensions
        let contract = parts
            .extensions
            .get::<Address>()
            .copied()
            .unwrap_or_default();

        // Filter transactions that target our contract
        let transactions = block
            .transactions
            .iter()
            .filter(|tx| tx.to.map_or(false, |to| to == contract))
            .cloned()
            .collect();

        Ok(ContractTransactions {
            contract,
            transactions,
        })
    }
}

impl<Ctx> FromJobCallParts<Ctx> for ContractTransactions
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlock;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Extracts all transactions in a block that match a specific function signature
#[derive(Debug, Clone)]
pub struct FunctionTransactions {
    /// The target function signature (first 4 bytes of the keccak256 hash of the function signature)
    pub signature: [u8; 4],
    /// The matching transactions
    pub transactions: Vec<Transaction>,
}

impl_deref_vec!(FunctionTransactions: Transaction);

impl TryFrom<&mut JobCallParts> for FunctionTransactions {
    type Error = MissingBlock;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block = parts
            .extensions
            .get::<Block<Transaction>>()
            .ok_or(MissingBlock)?;

        // Get the function signature from extensions
        let signature = parts
            .extensions
            .get::<[u8; 4]>()
            .copied()
            .unwrap_or_default();

        // Filter transactions that call our function
        let transactions = block
            .transactions
            .iter()
            .filter(|tx| {
                tx.input
                    .as_ref()
                    .map_or(false, |input| input.starts_with(&signature))
            })
            .cloned()
            .collect();

        Ok(FunctionTransactions {
            signature,
            transactions,
        })
    }
}

impl<Ctx> FromJobCallParts<Ctx> for FunctionTransactions
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlock;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Extracts all transactions in a block that target a specific contract and function
#[derive(Debug, Clone)]
pub struct ContractFunctionTransactions {
    /// The target contract address
    pub contract: Address,
    /// The target function signature
    pub signature: [u8; 4],
    /// The matching transactions
    pub transactions: Vec<Transaction>,
}

impl_deref_vec!(ContractFunctionTransactions: Transaction);

impl TryFrom<&mut JobCallParts> for ContractFunctionTransactions {
    type Error = MissingBlock;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block = parts
            .extensions
            .get::<Block<Transaction>>()
            .ok_or(MissingBlock)?;

        // Get the contract address and function signature from extensions
        let contract = parts
            .extensions
            .get::<Address>()
            .copied()
            .unwrap_or_default();
        let signature = parts
            .extensions
            .get::<[u8; 4]>()
            .copied()
            .unwrap_or_default();

        // Filter transactions that target our contract and function
        let transactions = block
            .transactions
            .iter()
            .filter(|tx| {
                tx.to.map_or(false, |to| to == contract)
                    && tx
                        .input
                        .as_ref()
                        .map_or(false, |input| input.starts_with(&signature))
            })
            .cloned()
            .collect();

        Ok(ContractFunctionTransactions {
            contract,
            signature,
            transactions,
        })
    }
}

impl<Ctx> FromJobCallParts<Ctx> for ContractFunctionTransactions
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlock;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;

    #[test]
    fn test_contract_transactions_extraction() {
        let mut parts = JobCallParts::new(0);
        let contract = Address::from([1u8; 20]);
        let tx = Transaction {
            to: Some(contract),
            ..Default::default()
        };
        let block = Block {
            transactions: vec![tx],
            ..Default::default()
        };

        parts.extensions.insert(block);
        parts.extensions.insert(contract);

        let result = ContractTransactions::try_from(&mut parts).unwrap();
        assert_eq!(result.contract, contract);
        assert_eq!(result.transactions.len(), 1);
    }

    #[test]
    fn test_function_transactions_extraction() {
        let mut parts = JobCallParts::new(0);
        let signature = [0xde, 0xad, 0xbe, 0xef];
        let tx = Transaction {
            input: Some(Bytes::from_iter(signature.iter().chain([0u8; 32].iter()))),
            ..Default::default()
        };
        let block = Block {
            transactions: vec![tx],
            ..Default::default()
        };

        parts.extensions.insert(block);
        parts.extensions.insert(signature);

        let result = FunctionTransactions::try_from(&mut parts).unwrap();
        assert_eq!(result.signature, signature);
        assert_eq!(result.transactions.len(), 1);
    }
}
