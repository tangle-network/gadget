#![allow(unused_parens)]

struct NotIntoJobResult;

#[blueprint_macros::debug_job]
async fn job() -> (NotIntoJobResult) {
    panic!()
}

fn main() {}
