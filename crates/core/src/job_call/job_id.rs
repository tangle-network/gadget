//! Job Identifiers and Conversion between them.
//!
//! A Job Identifier is a unique identifier for each job registered with the system.
//! It is used to identify a job and to route job calls to the appropriate job handler.
//!
//! A Job Id can be anything that implements `Into<JobId>`, and it is implemented for various primitive types.

use alloc::string::ToString;

/// Helper trait to convert types into `JobId`.
///
/// This trait can be implemented by types that can be converted into a `JobId`.
/// Any type that implements `Into<JobId>` automatically implements this trait.
///
/// # Example
/// ```
/// use blueprint_core::IntoJobId;
/// use blueprint_core::JobId;
///
/// let num: u64 = 42;
/// let job_id = num.into_job_id(); // Convert u64 into JobId
/// ```
pub trait IntoJobId {
    fn into_job_id(self) -> JobId;
}

impl<T: Into<JobId>> IntoJobId for T {
    fn into_job_id(self) -> JobId {
        self.into()
    }
}

/// Job Identifier.
#[derive(Clone, PartialEq, Eq, Hash, Copy)]
#[repr(C)]
pub struct JobId(pub [u64; 4]);

impl core::fmt::Debug for JobId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("JobId").field(&self.to_string()).finish()
    }
}

impl core::fmt::Display for JobId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for limb in self.0.iter() {
            write!(f, "{:016x}", limb)?;
        }
        Ok(())
    }
}

impl Default for JobId {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl JobId {
    /// The value zero. This is the only value that exists in all [`JobId`]
    /// types.
    pub const ZERO: Self = Self([0; 4]);

    /// The smallest value that can be represented by this integer type.
    /// Synonym for [`Self::ZERO`].
    pub const MIN: Self = Self::ZERO;

    /// The largest value that can be represented by this integer type,
    /// $2^{\mathtt{4}} − 1$.
    pub const MAX: Self = {
        let mut limbs = [u64::MAX; 4];
        limbs[4 - 1] &= u64::MAX;
        Self(limbs)
    };
}

macro_rules! impl_from_numbers {
	(
		$($ty:ty),*
	) => {
		$(
			impl From<$ty> for JobId {
				#[inline]
				fn from(value: $ty) -> Self {
					let mut id = [0; 4];
					id[3] = value as u64;
					Self(id)
				}
			}

			impl From<&$ty> for JobId {
				#[inline]
				fn from(value: &$ty) -> Self {
					Self::from(*value)
				}
			}
		)*
	};

	($(wide $ty:ty),*) => {
		$(
			impl From<$ty> for JobId {
				#[inline]
				fn from(value: $ty) -> Self {
					let mut id = [0; 4];
					id[3] = value as u64;
					id[2] = (value >> 64) as u64;
					Self(id)
				}
			}

			impl From<&$ty> for JobId {
				#[inline]
				fn from(value: &$ty) -> Self {
					Self::from(*value)
				}
			}
		)*
	};
}

impl_from_numbers!(u8, i8, u16, i16, u32, i32, u64, i64, usize, isize);
impl_from_numbers!(wide u128, wide i128);

impl From<[u8; 32]> for JobId {
    #[inline]
    fn from(value: [u8; 32]) -> Self {
        // Safe because they are same size and layout
        Self(unsafe { core::mem::transmute::<[u8; 32], [u64; 4]>(value) })
    }
}

impl From<&[u8]> for JobId {
    #[inline]
    fn from(value: &[u8]) -> Self {
        use tiny_keccak::Hasher;
        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(value);
        let mut out = [0u8; 32];
        hasher.finalize(&mut out);
        Self::from(out)
    }
}

impl From<&str> for JobId {
    #[inline]
    fn from(value: &str) -> Self {
        Self::from(value.as_bytes())
    }
}

impl From<alloc::string::String> for JobId {
    #[inline]
    fn from(value: alloc::string::String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&alloc::string::String> for JobId {
    #[inline]
    fn from(value: &alloc::string::String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<alloc::vec::Vec<u8>> for JobId {
    #[inline]
    fn from(value: alloc::vec::Vec<u8>) -> Self {
        Self::from(&value)
    }
}

impl From<&alloc::vec::Vec<u8>> for JobId {
    #[inline]
    fn from(value: &alloc::vec::Vec<u8>) -> Self {
        Self::from(value.as_slice())
    }
}

impl From<()> for JobId {
    #[inline]
    fn from(_: ()) -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_id_works() {
        let id = JobId::from("test");
        assert_eq!(id, JobId::from("test"));
        assert_ne!(id, JobId::from("test2"));
    }
}
