pub mod data {
    pub mod api {
        pub mod adwmh;
        pub mod client;
        pub mod jwxt;
        pub mod ycard;
    }
    pub mod auth;
    pub mod crawler;
    pub mod model;
    pub mod session;
}

pub mod utils {
    pub mod des;
    pub mod parser;
}

mod diagnostics;

#[cfg(test)]
mod diagnostic_safety_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod updater;

mod ffi;
mod persistence;

#[cfg(target_os = "android")]
pub mod jni;

pub mod core;
#[cfg(feature = "server")]
pub mod server;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
