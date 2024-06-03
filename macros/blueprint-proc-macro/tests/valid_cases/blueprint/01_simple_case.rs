use gadget_blueprint_proc_macro::blueprint;

blueprint! {
    x: 100,
    y: 0,
}

fn main() {
    use blueprint::*;
    println!("{:?}", BLUEPRINT);
}
