use gadget_blueprint_proc_macro::job;
use std::convert::Infallible;

#[job(id = 0, params(n, vec), result(Vec<u8>, String))]
fn keygen(n: u16, vec: Vec<u8>) -> Result<(Vec<u8>, String), Infallible> {
    let _ = (n, vec);
    Ok((Vec::new(), String::new()))
}

fn main() {
    println!("{KEYGEN_JOB_DEF}");
}
