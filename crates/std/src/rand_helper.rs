use rand::RngExt;

#[cfg(not(feature = "std"))]
use rand::prelude::StdRng;

pub use rand::{
    self, CryptoRng, Rng, TryCryptoRng, TryRng,
    distr::{Distribution, StandardUniform},
};

#[cfg(feature = "std")]
pub use rand::rngs::SysRng;

/// Trait for generating uniform random values
pub trait UniformRand: Sized {
    fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self;
}

impl<T> UniformRand for T
where
    StandardUniform: Distribution<T>,
{
    #[inline]
    fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.sample(StandardUniform)
    }
}

/// A random number generator that works in both std and `no_std` environments
#[derive(Debug)]
pub struct BlueprintRng(RngImpl);

#[cfg(feature = "std")]
type RngImpl = rand::rngs::SysRng;

#[cfg(not(feature = "std"))]
type RngImpl = StdRng;

impl BlueprintRng {
    /// Create a new cryptographically secure random number generator
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "std")]
        {
            Self(SysRng)
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
            // In std mode, always use the OS RNG regardless of seed for security
            let _ = seed;
            Self(SysRng)
        }
        #[cfg(not(feature = "std"))]
        {
            use rand::SeedableRng;
            Self(StdRng::from_seed(seed))
        }
    }
}

impl Default for BlueprintRng {
    fn default() -> Self {
        Self::new()
    }
}

// `TryCryptoRng` is a marker with no blanket impl for these newtypes, so it is written out
// for both feature sets; `CryptoRng` then follows for free wherever `<Self as TryRng>::Error`
// is `Infallible` (both arms below), since `rand_core` blanket-impls it on that bound.
impl rand::TryCryptoRng for BlueprintRng {}

#[cfg(feature = "std")]
impl rand::TryRng for BlueprintRng {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self
            .0
            .try_next_u32()
            .unwrap_or_else(|_| unreachable!("OS RNG is available")))
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self
            .0
            .try_next_u64()
            .unwrap_or_else(|_| unreachable!("OS RNG is available")))
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        self.0
            .try_fill_bytes(dest)
            .unwrap_or_else(|_| unreachable!("OS RNG is available"));
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl rand::TryRng for BlueprintRng {
    type Error = rand::rand_core::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        self.0.fill_bytes(dest);
        Ok(())
    }
}

/// An infallible view over [`rand::rngs::SysRng`]: the platform's OS entropy source with the
/// fallibility erased, so it satisfies `CryptoRng + Rng`. `SysRng` itself is a fallible
/// `TryRng`, which `rand` 0.10 no longer aliases to an infallible `OsRng`.
#[cfg(feature = "std")]
#[derive(Debug, Default)]
pub struct SystemRng(SysRng);

#[cfg(feature = "std")]
impl SystemRng {
    /// Wrap the OS entropy source.
    #[must_use]
    pub const fn new() -> Self {
        Self(SysRng)
    }
}

#[cfg(feature = "std")]
impl rand::TryCryptoRng for SystemRng {}

#[cfg(feature = "std")]
impl rand::TryRng for SystemRng {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.try_next_u32().expect("OS RNG is available"))
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.try_next_u64().expect("OS RNG is available"))
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        self.0.try_fill_bytes(dest).expect("OS RNG is available");
        Ok(())
    }
}

/// `rand_core` 0.6 compatibility shim for a [`SystemRng`], so crates still on the 0.6/0.8
/// `rand` API (e.g. `ed25519-zebra`) can use the same OS entropy source.
#[cfg(feature = "std")]
pub struct RandCore06(SystemRng);

#[cfg(feature = "std")]
impl RandCore06 {
    /// Wrap a [`SystemRng`].
    #[must_use]
    pub const fn new() -> Self {
        Self(SystemRng::new())
    }
}

#[cfg(feature = "std")]
impl Default for RandCore06 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl rand_core06::RngCore for RandCore06 {
    fn next_u32(&mut self) -> u32 {
        rand::Rng::next_u32(&mut self.0)
    }

    fn next_u64(&mut self) -> u64 {
        rand::Rng::next_u64(&mut self.0)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand::Rng::fill_bytes(&mut self.0, dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core06::Error> {
        rand::Rng::fill_bytes(&mut self.0, dest);
        Ok(())
    }
}

#[cfg(feature = "std")]
impl rand_core06::CryptoRng for RandCore06 {}

/// Create a deterministic RNG for testing
#[must_use]
pub fn test_rng() -> BlueprintRng {
    const TEST_SEED: [u8; 32] = [
        1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    BlueprintRng::from_seed(TEST_SEED)
}
