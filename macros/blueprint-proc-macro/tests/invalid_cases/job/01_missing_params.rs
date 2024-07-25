use gadget_blueprint_proc_macro::job;

#[job(id = 0, result(Vec<u8>))]
fn keygen(n: u16) -> Vec<u8> {
    Vec::new()
}

fn main() {}
