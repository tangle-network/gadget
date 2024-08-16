use gadget_sdk::job;
use std::convert::Infallible;

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    verifier(evm = "IncredibleSquaringBlueprint")
    event_handler_type("tangle"),
)]
pub fn xsquare(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 1,
    params(x),
    result(_),
    verifier(evm = "IncredibleSquaringBlueprint")
    event_handler_type("evm"),
)]
pub fn xcube(x: u64) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(3u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let x = 3;
        assert_eq!(xsquare(x).unwrap(), 9);

        let x = u64::MAX;
        assert_eq!(xsquare(x).unwrap(), u64::MAX);
    }
}
