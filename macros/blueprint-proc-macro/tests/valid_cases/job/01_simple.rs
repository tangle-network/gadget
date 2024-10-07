use std::convert::Infallible;

use gadget_blueprint_proc_macro::job;

/// A simple job that generates a key of length `n`
#[job(id = 0, params(n), result(Vec<u8>))]
fn keygen(n: u16) -> Result<Vec<u8>, Infallible> {
    let _ = n;
    Ok(Vec::new())
}

fn main() {
    // Ensure the generated code exists
    println!("{KEYGEN_JOB_DEF}");
}
