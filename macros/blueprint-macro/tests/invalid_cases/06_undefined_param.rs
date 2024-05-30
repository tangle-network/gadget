use gadget_blueprint_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n, t), result(Bytes))]
fn keygen(n: u16, vec: Vec<u8>) -> Result<Bytes, String> {
    let _ = (n, vec);
    Err(String::new())
}

fn main() {}
