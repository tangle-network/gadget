use gadget_context_derive::KeystoreContext;

#[derive(KeystoreContext)]
enum MyContext {
    Variant1,
    Variant2,
}

fn main() {}
