use gadget_blueprint_proc_macro::job;
use std::convert::Infallible;

#[job(id = 0, params(n), result(Vec<u8>, String))]
fn keygen(n: u16) -> Result<(Vec<u8>, String), Infallible> {
    let _ = n;
    Ok((Vec::new(), String::new()))
}

fn main() {
    println!("{KEYGEN_JOB_DEF}");
}
