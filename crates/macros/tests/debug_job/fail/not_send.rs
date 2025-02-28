use blueprint_macros::debug_job;

#[debug_job]
async fn job() {
    let _rc = std::rc::Rc::new(());
    async {}.await;
}

fn main() {}
