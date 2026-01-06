pub mod data {
    pub mod api {
        pub mod client;
        pub mod jwxt;
        pub mod adwmh;
        pub mod ycard;
    }
    pub mod model;
    pub mod crawler;
    pub mod auth;
}

pub mod utils {
    pub mod des;
    pub mod parser;
}

pub mod updater;

#[cfg(target_os = "android")]
pub mod jni;
