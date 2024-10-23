use gadget_blueprint_proc_macro::job;
use std::convert::Infallible;

#[job(id = 0, params(n, s), result(_))]
fn keygen(n: u16, s: String) -> Result<Vec<u8>, Infallible> {
    let _ = (n, s);
    Ok(Vec::new())
}

fn main() {
    println!("{KEYGEN_JOB_DEF}");
}
