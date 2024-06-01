use alloy_network::{EthereumSigner, Network};
use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use alloy_provider::Provider;

use alloy_signer::k256::ecdsa;
use alloy_signer::k256::ecdsa::signature::Keypair;
use alloy_signer::utils::raw_public_key_to_address;
use alloy_signer::Signer;
use alloy_transport::Transport;
use eigen_contracts::{BlsApkRegistry, OperatorStateRetriever, RegistryCoordinator, StakeRegistry};

use crate::crypto::bls::{G1Point, KeyPair};
use crate::el_contracts::reader::ElChainReader;
use crate::el_contracts::reader::ElReader;
use crate::types::{*};

type AvsRegistryWriterResult<T> = Result<T, AvsError>;

pub struct AvsRegistryChainWriter<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Copy + 'static,
    N: Network,
    S: Signer + Send + Sync,
{
    service_manager_addr: Address,
    registry_coordinator: RegistryCoordinator::RegistryCoordinatorInstance<T, P, N>,
    operator_state_retriever: OperatorStateRetriever::OperatorStateRetrieverInstance<T, P, N>,
    stake_registry: StakeRegistry::StakeRegistryInstance<T, P, N>,
    bls_apk_registry: BlsApkRegistry::BlsApkRegistryInstance<T, P, N>,
    el_reader: ElChainReader<T, P, N>,
    // logger: Logger,
    eth_client: P,
    tx_mgr: EthereumSigner,
    signer: S,
}

impl<T, P, N, S> AvsRegistryChainWriter<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Copy + 'static,
    N: Network,
    S: Signer + Send + Sync,
{
    pub fn new(
        service_manager_addr: Address,
        registry_coordinator: RegistryCoordinator::RegistryCoordinatorInstance<T, P, N>,
        operator_state_retriever: OperatorStateRetriever::OperatorStateRetrieverInstance<T, P, N>,
        stake_registry: StakeRegistry::StakeRegistryInstance<T, P, N>,
        bls_apk_registry: BlsApkRegistry::BlsApkRegistryInstance<T, P, N>,
        el_reader: ElChainReader<T, P, N>,
        // logger: Logger,
        eth_client: P,
        tx_mgr: EthereumSigner,
        signer: S,
    ) -> Self {
        Self {
            service_manager_addr,
            registry_coordinator,
            operator_state_retriever,
            stake_registry,
            bls_apk_registry,
            el_reader,
            // logger,
            eth_client,
            tx_mgr,
            signer,
        }
    }

    pub async fn register_operator_in_quorum_with_avs_registry_coordinator(
        &self,
        operator_ecdsa_private_key: &ecdsa::SigningKey,
        operator_to_avs_registration_sig_salt: FixedBytes<32>,
        operator_to_avs_registration_sig_expiry: U256,
        bls_key_pair: &KeyPair,
        quorum_numbers: Bytes,
        socket: String,
    ) -> AvsRegistryWriterResult<<N as Network>::ReceiptResponse> {
        let operator_addr = raw_public_key_to_address(
            operator_ecdsa_private_key
                .verifying_key()
                .to_sec1_bytes()
                .as_ref(),
        );

        // self.logger
        //     .info("Registering operator with the AVS's registry coordinator");

        let g1_hashed_msg_to_sign = self
            .registry_coordinator
            .pubkeyRegistrationMessageHash(operator_addr)
            .call()
            .await
            .map(|x| x._0)
            .map_err(AvsError::from)?;

        let signed_msg = bls_key_pair.sign_hashed_to_curve_message(&G1Point {
            x: g1_hashed_msg_to_sign.X,
            y: g1_hashed_msg_to_sign.Y,
        });
        let g1_pubkey_bn254 = bls_key_pair.get_pub_key_g1();
        let g2_pubkey_bn254 = bls_key_pair.get_pub_key_g2();

        let pubkey_reg_params = RegistryCoordinator::PubkeyRegistrationParams {
            pubkeyRegistrationSignature: RegistryCoordinator::G1Point {
                X: signed_msg.g1_point.x,
                Y: signed_msg.g1_point.y,
            },
            pubkeyG1: RegistryCoordinator::G1Point {
                X: g1_pubkey_bn254.x,
                Y: g1_pubkey_bn254.y,
            },
            pubkeyG2: RegistryCoordinator::G2Point {
                X: g2_pubkey_bn254.x,
                Y: g2_pubkey_bn254.y,
            },
        };

        let msg_to_sign = self
            .el_reader
            .calculate_operator_avs_registration_digest_hash(
                operator_addr,
                self.service_manager_addr,
                operator_to_avs_registration_sig_salt,
                operator_to_avs_registration_sig_expiry,
            )
            .await?;

        let operator_signature = self
            .signer
            .sign_message(msg_to_sign.as_ref())
            .await
            .map_err(AvsError::from)?;

        let operator_signature_with_salt_and_expiry =
            RegistryCoordinator::SignatureWithSaltAndExpiry {
                signature: Bytes::copy_from_slice(operator_signature.as_bytes().as_ref()),
                salt: operator_to_avs_registration_sig_salt,
                expiry: operator_to_avs_registration_sig_expiry,
            };

        let receipt = self
            .registry_coordinator
            .registerOperator(
                quorum_numbers,
                socket,
                pubkey_reg_params,
                operator_signature_with_salt_and_expiry,
            )
            .send()
            .await?
            .get_receipt()
            .await?;

        // self.logger
        //     .info("Successfully registered operator with AVS registry coordinator");

        Ok(receipt)
    }

    pub async fn update_stakes_of_entire_operator_set_for_quorums(
        &self,
        operators_per_quorum: Vec<Vec<Address>>,
        quorum_numbers: Bytes,
    ) -> AvsRegistryWriterResult<<N as Network>::ReceiptResponse> {
        // self.logger.info("Updating stakes for entire operator set");

        let receipt = self
            .registry_coordinator
            .updateOperatorsForQuorum(operators_per_quorum, quorum_numbers)
            .send()
            .await?
            .get_receipt()
            .await?;

        // self.logger
        //     .info("Successfully updated stakes for entire operator set");

        Ok(receipt)
    }

    pub async fn update_stakes_of_operator_subset_for_all_quorums(
        &self,
        operators: Vec<Address>,
    ) -> AvsRegistryWriterResult<<N as Network>::ReceiptResponse> {
        // self.logger
        //     .info("Updating stakes of operator subset for all quorums");

        let receipt = self
            .registry_coordinator
            .updateOperators(operators)
            .send()
            .await?
            .get_receipt()
            .await?;

        // self.logger
        //     .info("Successfully updated stakes of operator subset for all quorums");

        Ok(receipt)
    }

    pub async fn deregister_operator(
        &self,
        quorum_numbers: Bytes,
    ) -> AvsRegistryWriterResult<<N as Network>::ReceiptResponse> {
        // self.logger
        //     .info("Deregistering operator with the AVS's registry coordinator");

        let receipt = self
            .registry_coordinator
            .deregisterOperator(quorum_numbers)
            .send()
            .await?
            .get_receipt()
            .await?;

        // self.logger
        //     .info("Successfully deregistered operator with the AVS's registry coordinator");

        Ok(receipt)
    }
}
