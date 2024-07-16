use alloy_primitives::U256;

pub const X_SQUARE_JOB_ID: u8 = 0;
/// Returns x^2 saturating to [`U256::MAX`] if overflow occurs.
pub fn xsquare(x: U256) -> U256 {
    x.saturating_pow(U256::from(2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    #[test]
    fn it_works() {
        let x = U256::from(3);
        assert_eq!(xsquare(x), U256::from(9));

        let x = U256::MAX;
        assert_eq!(xsquare(x), U256::MAX);
    }
}
