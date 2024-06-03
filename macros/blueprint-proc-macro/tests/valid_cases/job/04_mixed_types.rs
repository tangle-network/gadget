use gadget_blueprint_proc_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n, s), result(Bytes))]
fn keygen(n: u16, s: String) -> Bytes {
    let _ = (n, s);
    Bytes::new()
}

fn main() {
    // Ensure the generated struct exists
    let _ = KeygenJob { params: (16u16, String::new()), result: Bytes::new() };
}
