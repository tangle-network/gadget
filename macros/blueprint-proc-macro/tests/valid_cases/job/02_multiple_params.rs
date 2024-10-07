use gadget_blueprint_proc_macro::job;
use std::convert::Infallible;

#[job(id = 0, params(n, t), result(Vec<u8>))]
fn keygen(n: u16, t: u8) -> Result<Vec<u8>, Infallible> {
    let _ = (n, t);
    Ok(Vec::new())
}

fn main() {
    println!("{KEYGEN_JOB_DEF}");
}
