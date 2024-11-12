use gadget_blueprint_proc_macro::job;

#[job(id = 0, params(n, t), event_listener(listener = null), result(Vec<u8>))]
fn keygen(n: u16, vec: Vec<u8>) -> Result<Vec<u8>, String> {
    let _ = (n, vec);
    Err(String::new())
}

fn main() {}
