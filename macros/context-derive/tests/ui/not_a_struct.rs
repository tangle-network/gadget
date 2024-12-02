use gadget_sdk::contexts::KeystoreContext;

#[derive(KeystoreContext)]
enum MyContext {
    Variant1,
    Variant2,
}

fn main() {}
