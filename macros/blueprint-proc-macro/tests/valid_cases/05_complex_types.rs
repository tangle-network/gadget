use gadget_blueprint_proc_macro::job;

struct Bytes;
impl Bytes {
    fn new() -> Self {
        Self
    }
}

#[job(params(n, vec), result(Bytes, String))]
fn keygen(n: u16, vec: Vec<u8>) -> Result<(Bytes, String), String> {
    let _ = (n, vec);
    Err(String::new())
}

fn main() {
    // Ensure the generated struct exists
    let _ = Keygen {
        params: (16u16, vec![1u8, 2, 3]),
        result: (Bytes::new(), String::new()),
    };
}
