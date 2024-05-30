use gadget_blueprint_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n, t), result(Bytes))]
fn keygen(n: u16, t: u8) -> Bytes {
    let _ = (n, t);
    Bytes::new()
}

fn main() {
    // Ensure the generated struct exists
    let _ = Keygen { params: (16u16, 8u8), result: Bytes::new() };
}
