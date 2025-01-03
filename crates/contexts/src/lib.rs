#[cfg(feature = "eigenlayer")]
pub mod eigenlayer;
#[cfg(feature = "evm")]
pub mod instrumented_evm_client;
#[cfg(feature = "keystore")]
pub mod keystore;
#[cfg(feature = "networking")]
pub mod p2p;
#[cfg(feature = "tangle")]
pub mod services;
#[cfg(feature = "tangle")]
pub mod tangle;

#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}
