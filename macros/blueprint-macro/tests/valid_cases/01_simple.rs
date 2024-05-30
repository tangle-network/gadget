use gadget_blueprint_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n), result(Bytes))]
fn keygen(n: u16) -> Bytes {
    let _ = n;
    Bytes::new()
}

fn main() {
    // Ensure the generated struct exists
    let _ = Keygen {
        params: (16),
        result: Bytes::new(),
    };
}
