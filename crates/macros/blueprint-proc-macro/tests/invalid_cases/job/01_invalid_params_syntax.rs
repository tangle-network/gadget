use blueprint_sdk::macros::job;

#[job(id = 0, params(n t), result(Vec<u8>))]
fn keygen(n: u16, t: u8) -> Vec<u8> {
    Vec::new()
}

fn main() {}
