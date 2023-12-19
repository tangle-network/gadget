use crate::mock::*;

#[test]
fn gadget_starts() {
    let (mut ext, runtime) = new_test_ext();
    ext.execute_with(|| {
        assert_eq!(1, 1);
    });
}
