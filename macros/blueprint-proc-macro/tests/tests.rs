#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn test_valid_cases() {
        let t = TestCases::new();
        t.pass("tests/valid_cases/*.rs");
    }

    #[test]
    fn test_invalid_cases() {
        let t = TestCases::new();
        t.compile_fail("tests/invalid_cases/*.rs");
    }
}
