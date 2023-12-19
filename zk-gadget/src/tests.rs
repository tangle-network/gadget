use crate::mock::*;

#[test]
fn gadget_starts() {
    let (ext, runtime) = new_test_ext();
    ext.lock().execute_with(|| {
        assert_eq!(1, 1);
    });
}
