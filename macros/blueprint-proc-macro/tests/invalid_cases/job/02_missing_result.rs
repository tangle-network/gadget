use gadget_blueprint_proc_macro::job;

#[job(id = 0, params(n))]
fn keygen(n: u16) -> Vec<u8> {
    Vec::new()
}

fn main() {}
