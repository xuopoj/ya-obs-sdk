//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod config;
pub mod credentials;
pub mod error;
pub mod models;
pub mod signer;
pub mod url;
pub mod xml;

pub use config::{AddressingStyle, ClientConfig, SigningVersion};
pub use credentials::Credentials;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
