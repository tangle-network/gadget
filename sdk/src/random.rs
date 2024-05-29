//! Returns `OsRng` with `getrandom`, or a `CryptoRng` which panics without `getrandom`.

/// Re-export [`rand_core`] types to simplify dependences
pub use rand_core::{self, CryptoRng, CryptoRngCore, RngCore, SeedableRng};

/// Returns `OsRng` with `getrandom`, or a `CryptoRng` which panics without `getrandom`.
#[cfg(all(feature = "getrandom", not(feature = "std")))]
#[must_use]
pub fn getrandom_or_panic() -> impl RngCore + CryptoRng {
    rand_core::OsRng
}

/// Returns `OsRng` with `getrandom`, or a `CryptoRng` which panics without `getrandom`.
#[cfg(all(feature = "getrandom", feature = "std"))]
#[must_use]
pub fn getrandom_or_panic() -> impl RngCore + CryptoRng {
    rand::thread_rng()
}

/// Returns `OsRng` with `getrandom`, or a `CryptoRng` which panics without `getrandom`.
#[cfg(not(feature = "getrandom"))]
pub fn getrandom_or_panic() -> impl RngCore + CryptoRng {
    const PRM: &'static str = "Attempted to use functionality that requires system randomness!!";

    // TODO: Should we panic when invoked or when used?

    struct PanicRng;
    impl rand_core::RngCore for PanicRng {
        fn next_u32(&mut self) -> u32 {
            panic!("{}", PRM)
        }
        fn next_u64(&mut self) -> u64 {
            panic!("{}", PRM)
        }
        fn fill_bytes(&mut self, _dest: &mut [u8]) {
            panic!("{}", PRM)
        }
        fn try_fill_bytes(&mut self, _dest: &mut [u8]) -> Result<(), rand_core::Error> {
            panic!("{}", PRM)
        }
    }
    impl rand_core::CryptoRng for PanicRng {}

    PanicRng
}
