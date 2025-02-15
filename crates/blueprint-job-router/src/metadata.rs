//! Job Call Metadata is a collection of metadata that can be included in a job call to provide additional information to the job call.
//! or a Job Result to provide additional context for the execution results.

use core::convert::Infallible;
use core::fmt;
use core::str::FromStr;

use alloc::borrow::Cow;
use alloc::collections::btree_map::{Entry, Iter, IterMut};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use bytes::{Bytes, BytesMut};

/// A typed metadata map that stores key-value pairs where keys are static strings.
///
/// This structure wraps a `BTreeMap` with string keys and generic values, providing
/// a convenient way to store metadata with type safety. The keys are stored as
/// `Cow<'static, str>` to allow both owned and borrowed strings without allocation
/// when possible.
///
/// # Type Parameters
///
/// * `T`: The type of values stored in the metadata map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MetadataMap<T> {
    map: BTreeMap<Cow<'static, str>, T>,
}

impl<T> MetadataMap<T> {
    /// Creates a new empty `MetadataMap`.
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; it is retained as
    /// is (verbatim).
    pub fn insert<K, V>(&mut self, key: K, value: V) -> Option<T>
    where
        K: Into<Cow<'static, str>>,
        V: Into<T>,
    {
        self.map.insert(key.into(), value.into())
    }

    /// Gets a reference to the value associated with the given key.
    pub fn get<K>(&self, key: K) -> Option<&T>
    where
        K: AsRef<str>,
    {
        self.map.get(key.as_ref())
    }

    /// Gets a mutable reference to the value associated with the given key.
    pub fn get_mut<K>(&mut self, key: &K) -> Option<&mut T>
    where
        K: AsRef<str>,
    {
        self.map.get_mut(key.as_ref())
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    pub fn remove<K>(&mut self, key: &K) -> Option<T>
    where
        K: AsRef<str>,
    {
        self.map.remove(key.as_ref())
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns an iterator over the map's entries.
    pub fn iter(&self) -> Iter<'_, Cow<'static, str>, T> {
        self.map.iter()
    }

    /// Returns a mutable iterator over the map's entries.
    pub fn iter_mut(&mut self) -> IterMut<'_, Cow<'static, str>, T> {
        self.map.iter_mut()
    }

    /// Provides a view into a single entry in the map, which may or may not be present.
    pub fn entry<K>(&mut self, key: K) -> Entry<'_, Cow<'static, str>, T>
    where
        K: Into<Cow<'static, str>>,
    {
        self.map.entry(key.into())
    }

    /// Extends the map with the key-value pairs from the given map.
    pub fn extend(&mut self, other: Self) {
        self.map.extend(other.map);
    }
}

/// Represents a Job Call metadata field value.
///
/// To handle this, the `MetadataValue` is usable as a type and can be compared
/// with strings and implements `Debug`. A `to_str` method is provided that returns
/// an `Err` if the metadata value contains non-visible ASCII characters.
#[derive(Clone, Default)]
pub struct MetadataValue {
    inner: Bytes,
    is_sensitive: bool,
}

impl FromStr for MetadataValue {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: Bytes::copy_from_slice(s.as_bytes()),
            is_sensitive: false,
        })
    }
}

impl MetadataValue {
    /// Create a new `MetadataValue` from a string.
    pub fn from_bytes(value: Bytes) -> Self {
        Self {
            inner: value,
            is_sensitive: false,
        }
    }

    /// Create a new `MetadataValue` from a string.
    pub fn from_sensitive_str(value: &str) -> Self {
        Self {
            inner: Bytes::copy_from_slice(value.as_bytes()),
            is_sensitive: true,
        }
    }

    /// Create a new `MetadataValue` from a string.
    pub fn from_sensitive_bytes(value: Bytes) -> Self {
        Self {
            inner: value,
            is_sensitive: true,
        }
    }

    /// Returns true if the metadata value is sensitive.
    pub fn is_sensitive(&self) -> bool {
        self.is_sensitive
    }

    /// Returns the length of the metadata value.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the metadata value is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Converts the metadata value into a string if it contains valid UTF-8.
    pub fn to_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.inner)
    }

    /// Converts the metadata value into a string without checking if it contains valid UTF-8.
    /// # Safety
    /// See [`core::str::from_utf8_unchecked`]
    pub unsafe fn to_str_unchecked(&self) -> &str {
        core::str::from_utf8_unchecked(&self.inner)
    }

    /// Converts the metadata value into bytes.
    pub fn into_bytes(self) -> Bytes {
        self.inner
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner[..]
    }
}

impl AsRef<[u8]> for MetadataValue {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl fmt::Debug for MetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_sensitive {
            f.write_str("Sensitive")
        } else {
            f.write_str("\"")?;
            let mut from = 0;
            let bytes = self.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                if !is_visible_ascii(b) || b == b'"' {
                    if from != i {
                        f.write_str(unsafe { core::str::from_utf8_unchecked(&bytes[from..i]) })?;
                    }
                    if b == b'"' {
                        f.write_str("\\\"")?;
                    } else {
                        write!(f, "\\x{:x}", b)?;
                    }
                    from = i + 1;
                }
            }

            if from != bytes.len() {
                f.write_str(unsafe { core::str::from_utf8_unchecked(&bytes[from..]) })?;
            }
            f.write_str("\"")
        }
    }
}

impl From<&str> for MetadataValue {
    fn from(value: &str) -> Self {
        Self::from_str(value).unwrap()
    }
}

impl From<Bytes> for MetadataValue {
    fn from(value: Bytes) -> Self {
        Self::from_bytes(value)
    }
}

impl From<&Bytes> for MetadataValue {
    fn from(value: &Bytes) -> Self {
        Self::from_bytes(value.clone())
    }
}

impl From<BytesMut> for MetadataValue {
    fn from(value: BytesMut) -> Self {
        Self::from_bytes(value.freeze())
    }
}

impl From<&BytesMut> for MetadataValue {
    fn from(value: &BytesMut) -> Self {
        Self::from_bytes(value.clone().freeze())
    }
}

impl From<String> for MetadataValue {
    fn from(value: String) -> Self {
        Self::from_str(&value).unwrap()
    }
}

impl From<&String> for MetadataValue {
    fn from(value: &String) -> Self {
        Self::from_str(value).unwrap()
    }
}

impl From<&[u8]> for MetadataValue {
    fn from(value: &[u8]) -> Self {
        Self::from_bytes(Bytes::copy_from_slice(value))
    }
}

impl From<Vec<u8>> for MetadataValue {
    fn from(value: Vec<u8>) -> Self {
        Self::from_bytes(Bytes::from(value))
    }
}

impl From<&Vec<u8>> for MetadataValue {
    fn from(value: &Vec<u8>) -> Self {
        Self::from_bytes(Bytes::copy_from_slice(value))
    }
}

impl<const N: usize> From<[u8; N]> for MetadataValue {
    fn from(value: [u8; N]) -> Self {
        Self::from_bytes(Bytes::copy_from_slice(&value))
    }
}

impl<const N: usize> From<&[u8; N]> for MetadataValue {
    fn from(value: &[u8; N]) -> Self {
        Self::from_bytes(Bytes::copy_from_slice(value))
    }
}

macro_rules! impl_from_numbers {
    ($($t:ty),*) => {
        $(
            /// Converts a number into a metadata value by converting it to a big-endian byte array.
            impl From<$t> for MetadataValue {
                fn from(value: $t) -> Self {
                    Self::from_bytes(Bytes::copy_from_slice(&value.to_be_bytes()))
                }
            }

            impl From<&$t> for MetadataValue {
                fn from(value: &$t) -> Self {
                    Self::from_bytes(Bytes::copy_from_slice(&value.to_be_bytes()))
                }
            }
        )*

    };
}

macro_rules! impl_try_from_metadata_for_numbers {
    ($($t:ty),*) => {
        $(
            /// Tries to convert a metadata value into a number by parsing it as a big-endian byte array.
            impl core::convert::TryFrom<MetadataValue> for $t {
                type Error = core::array::TryFromSliceError;

                fn try_from(value: MetadataValue) -> Result<Self, Self::Error> {
                    let bytes = value.as_bytes();
                    let mut arr = [0; core::mem::size_of::<Self>()];
                    arr.copy_from_slice(bytes);
                    Ok(Self::from_be_bytes(arr))
                }
            }

            /// Tries to convert a metadata value into a number by parsing it as a big-endian byte array.
            impl core::convert::TryFrom<&MetadataValue> for $t {
                type Error = core::array::TryFromSliceError;

                fn try_from(value: &MetadataValue) -> Result<Self, Self::Error> {
                    let bytes = value.as_bytes();
                    let mut arr = [0; core::mem::size_of::<Self>()];
                    arr.copy_from_slice(bytes);
                    Ok(Self::from_be_bytes(arr))
                }
            }
        )*
    };
}

impl_from_numbers! { u16, u32, u64, u128, usize, i16, i32, i64, i128, isize }
impl_try_from_metadata_for_numbers! { u16, u32, u64, u128, usize, i16, i32, i64, i128, isize }

const fn is_visible_ascii(b: u8) -> bool {
    b >= 32 && b < 127 || b == b'\t'
}
