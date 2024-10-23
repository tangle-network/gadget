use gadget_blueprint_proc_macro::job;
use std::convert::Infallible;

#[job(id = 300, params(n), result(Vec<u8>))]
fn keygen(n: u16) -> Result<Vec<u8>, Infallible> {
    let _ = n;
    Ok(Vec::new())
}

fn main() {}
