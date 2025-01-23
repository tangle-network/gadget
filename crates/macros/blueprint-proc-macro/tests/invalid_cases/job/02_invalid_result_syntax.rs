use blueprint_sdk::macros::job;

#[job(id = 0, params(n), result(Vec<u8> String))]
fn keygen(n: u16) -> (Vec<u8>, String) {
    (Vec::new(), String::new())
}

fn main() {}
