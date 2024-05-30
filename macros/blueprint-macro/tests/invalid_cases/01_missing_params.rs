use gadget_blueprint_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(result(Bytes))]
fn keygen(n: u16) -> Bytes {
    Bytes::new()
}

fn main() {}
