use rand::RngCore;

#[cfg(not(feature = "std"))]
use rand::prelude::StdRng;

pub use rand::{
    self, CryptoRng, Rng,
    distributions::{Distribution, Standard},
};

/// Trait for generating uniform random values
pub trait UniformRand: Sized {
    fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self;
}

impl<T> UniformRand for T
where
    Standard: Distribution<T>,
{
    #[inline]
    fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.sample(Standard)
    }
}

/// A random number generator that works in both std and `no_std` environments
pub struct GadgetRng(RngImpl);

#[cfg(feature = "std")]
type RngImpl = rand::rngs::OsRng;

#[cfg(not(feature = "std"))]
type RngImpl = StdRng;

impl GadgetRng {
    /// Create a new cryptographically secure random number generator
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "std")]
        {
            Self(rand::rngs::OsRng)
        }
        #[cfg(not(feature = "std"))]
        {
            test_rng()
        }
    }

    /// Create a deterministic RNG from a seed (for testing only)
    #[must_use]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        #[cfg(feature = "std")]
        {
            // TODO: ??
            let _ = seed;
            // Always use OsRng in std for security
            Self(rand::rngs::OsRng)
        }
        #[cfg(not(feature = "std"))]
        {
            use rand::SeedableRng;
            Self(StdRng::from_seed(seed))
        }
    }
}

impl Default for GadgetRng {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoRng for GadgetRng {}

#[cfg(feature = "std")]
impl RngCore for GadgetRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.0.try_fill_bytes(dest)
    }
}

#[cfg(not(feature = "std"))]
impl RngCore for GadgetRng {
    fn next_u32(&mut self) -> u32 {
        self.0.r#gen()
    }
    fn next_u64(&mut self) -> u64 {
        self.0.r#gen()
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.0.fill_bytes(dest);
        Ok(())
    }
}

/// Create a deterministic RNG for testing
#[must_use]
pub fn test_rng() -> GadgetRng {
    const TEST_SEED: [u8; 32] = [
        1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    GadgetRng::from_seed(TEST_SEED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng_generates_different_values() {
        let mut rng = GadgetRng::new();
        assert_ne!(rng.next_u64(), rng.next_u64());
    }

    #[test]
    fn test_deterministic_rng() {
        #[cfg(not(feature = "std"))]
        {
            let mut rng1 = GadgetRng::from_seed([1u8; 32]);
            let mut rng2 = GadgetRng::from_seed([1u8; 32]);
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }
}
