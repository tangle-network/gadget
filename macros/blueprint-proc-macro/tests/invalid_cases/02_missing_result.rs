use gadget_blueprint_proc_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n))]
fn keygen(n: u16) -> Bytes {
    Bytes::new()
}

fn main() {}
