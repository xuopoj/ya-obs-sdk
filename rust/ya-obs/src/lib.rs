//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod client;
pub mod config;
pub mod credentials;
pub mod error;
pub mod http;
pub mod models;
pub mod multipart;
pub mod operations;
pub mod retry;
pub mod signer;
pub mod streaming;
pub mod url;
pub mod xml;

pub use client::Client;
pub use config::{AddressingStyle, ClientConfig, SigningVersion};
pub use credentials::Credentials;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
