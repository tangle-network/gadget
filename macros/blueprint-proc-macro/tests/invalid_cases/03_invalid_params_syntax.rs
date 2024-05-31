use gadget_blueprint_proc_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n t), result(Bytes))]
fn keygen(n: u16, t: u8) -> Bytes {
    Bytes::new()
}

fn main() {}
