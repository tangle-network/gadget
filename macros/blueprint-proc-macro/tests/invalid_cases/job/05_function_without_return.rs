use gadget_blueprint_proc_macro::job;

#[job(id = 0, params(n), result(_))]
fn keygen(n: u16) {}

fn main() {}
