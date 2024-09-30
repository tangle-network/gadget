#[cfg(test)]
mod tests {
    #[test]
    fn test_derive_keystore_context() {
        let t = trybuild::TestCases::new();
        t.pass("tests/ui/01_basic.rs");
        t.pass("tests/ui/02_unnamed_fields.rs");
        t.pass("tests/ui/03_generic_struct.rs");
        t.compile_fail("tests/ui/04_missing_config_attr.rs");
        t.compile_fail("tests/ui/05_not_a_struct.rs");
        t.compile_fail("tests/ui/06_unit_struct.rs");
    }
}
