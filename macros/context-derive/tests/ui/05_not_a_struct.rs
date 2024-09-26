use gadget_sdk::ctx::KeystoreContext;

#[derive(KeystoreContext)]
enum MyContext {
    Variant1,
    Variant2,
}

fn main() {}
