//! ya-obs — Rust SDK for Huawei Cloud OBS.

pub mod error;
pub mod models;
pub mod signer;
pub mod xml;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
