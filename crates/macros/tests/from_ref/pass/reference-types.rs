#![deny(noop_method_call)]

use blueprint_macros::FromRef;

#[derive(FromRef)]
struct MyContext {
    inner: &'static str,
}

fn main() {}
