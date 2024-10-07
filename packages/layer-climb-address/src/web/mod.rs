mod shared;
pub use shared::*;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        mod keplr;
        pub use keplr::*;
    } else {
        mod dummy;
        pub use dummy::*;
    }
}
