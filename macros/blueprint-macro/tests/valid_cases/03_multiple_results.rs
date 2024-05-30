use gadget_blueprint_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n), result(Bytes, String))]
fn keygen(n: u16) -> (Bytes, String) {
    let _ = n;
    (Bytes::new(), String::new())
}

fn main() {
    // Ensure the generated struct exists
    let _ = Keygen { params: (16u16), result: (Bytes::new(), String::new()) };
}
