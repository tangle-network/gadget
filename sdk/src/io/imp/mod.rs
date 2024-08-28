cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        mod standard;
        pub use standard::keystore::*;
        pub use standard::shell::*;
    } else {
        mod no_std;
        pub use no_std::keystore::*;
        pub use no_std::shell::*;
    }
}
