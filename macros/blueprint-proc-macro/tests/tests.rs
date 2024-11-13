/// Add valid cases under a mod to prove the syntax compiles
/// We do not do the same for invalid cases since we are proving
/// there that the syntax does not compile
pub mod valid_cases;

#[derive(Clone)]
struct EmptyContext;

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn test_jobs_invalid_cases() {
        let t = TestCases::new();
        t.compile_fail("tests/invalid_cases/job/*.rs");
    }
}
