use gadget_blueprint_proc_macro::job;
use gadget_blueprint_proc_macro_core::*;

#[job(params(n), result(Vec<u8>))]
fn keygen(n: u16) -> Vec<u8> {
    let _ = n;
    Vec::new()
}

fn main() {
    // Ensure the generated struct exists
    // let _ = KeygenJob {
    //     params: (16),
    //     result: Bytes::new(),
    // };
}
