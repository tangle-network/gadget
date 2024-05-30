use gadget_blueprint_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n), result(Bytes String))]
fn keygen(n: u16) -> (Bytes, String) {
    (Bytes::new(), String::new())
}

fn main() {}
