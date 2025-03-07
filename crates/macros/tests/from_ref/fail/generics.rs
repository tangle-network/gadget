use blueprint_sdk::extract::FromRef;

#[derive(Clone, FromRef)]
struct AppContext<T> {
    foo: T,
}

fn main() {}
