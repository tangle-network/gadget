use gadget_blueprint_proc_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n), result(_))]
fn keygen(n: u16) -> Bytes {
    let _ = n;
    Bytes::new()
}

fn main() {
    // Ensure the generated struct exists
    let _ = KeygenJob {
        params: (16),
        result: Bytes::new(),
    };
}
