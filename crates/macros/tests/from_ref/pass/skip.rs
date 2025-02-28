use blueprint_macros::FromRef;

#[derive(Clone, FromRef)]
struct MyContext {
    auth_token: String,
    #[from_ref(skip)]
    also_string: String,
}

fn main() {}
