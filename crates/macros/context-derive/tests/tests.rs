mod ui;

#[cfg(test)]
mod tests {
    #[test]
    fn test_derive_context() {
        let t = trybuild::TestCases::new();
        t.pass("tests/ui/basic.rs");
        t.pass("tests/ui/unnamed_fields.rs");
        t.pass("tests/ui/generic_struct.rs");
        t.compile_fail("tests/ui/missing_config_attr.rs");
        t.compile_fail("tests/ui/not_a_struct.rs");
        t.compile_fail("tests/ui/unit_struct.rs");
    }
}
