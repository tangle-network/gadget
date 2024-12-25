pub mod config;
pub mod error;
pub mod executor;
pub mod gadget;
pub mod protocols;
pub mod sdk;
pub mod sources;
pub use executor::run_blueprint_manager;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
