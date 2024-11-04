use gadget_blueprint_proc_macro::job;
use std::convert::Infallible;

#[job(id = 0, params(n))]
fn keygen(n: u16) -> Result<Vec<u8>, Infallible> {
    let _ = n;
    Ok(Vec::new())
}

fn main() {
    println!("{KEYGEN_JOB_DEF}");
}
