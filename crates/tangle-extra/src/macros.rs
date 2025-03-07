/// Counts the number of identifiers provided to the macro.
///
/// This macro recursively counts the number of identifiers passed to it by
/// processing each identifier and the rest of the token tree separately.
///
/// # Examples
///
/// ```
/// # use blueprint_tangle_extra::count;
/// let c = count!(a, b, c, d);
/// assert_eq!(c, 4);
///
/// let empty_count = count!();
/// assert_eq!(empty_count, 0);
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! count {
    ($val:ident, $($rest:tt)*) => {
        1 + $crate::count!($($rest)*)
    };
    ($val:ident) => {
        1
    };
    () => {
        0
    }
}
