use thiserror::Error;

use crate::xml::parse_error_response;

#[derive(Debug, Error)]
pub enum Error {
    #[error("NoSuchKey: {message} (request_id={request_id})")]
    NoSuchKey { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("NoSuchBucket: {message} (request_id={request_id})")]
    NoSuchBucket { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("AccessDenied: {message} (request_id={request_id})")]
    AccessDenied { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("client error {status} {code}: {message}")]
    Client { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("server error {status} {code}: {message}")]
    Server { code: String, message: String, status: u16, request_id: String, host_id: String },

    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("config error: {0}")]
    Config(String),
}

impl Error {
    pub fn code(&self) -> &str {
        match self {
            Error::NoSuchKey { code, .. }
            | Error::NoSuchBucket { code, .. }
            | Error::AccessDenied { code, .. }
            | Error::Client { code, .. }
            | Error::Server { code, .. } => code,
            Error::Transport(_) => "Transport",
            Error::Xml(_) => "XmlParse",
            Error::Config(_) => "Config",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Error::NoSuchKey { message, .. }
            | Error::NoSuchBucket { message, .. }
            | Error::AccessDenied { message, .. }
            | Error::Client { message, .. }
            | Error::Server { message, .. } => message,
            Error::Transport(_) => "transport error",
            Error::Xml(_) => "xml parse error",
            Error::Config(_) => "config error",
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            Error::NoSuchKey { status, .. }
            | Error::NoSuchBucket { status, .. }
            | Error::AccessDenied { status, .. }
            | Error::Client { status, .. }
            | Error::Server { status, .. } => *status,
            _ => 0,
        }
    }
}

/// Map an HTTP status + OBS XML error body to the right `Error` variant.
pub fn classify_error(status: u16, xml: &str) -> Error {
    let parsed = match parse_error_response(xml) {
        Ok(p) => p,
        Err(e) => return Error::Xml(e),
    };

    match (parsed.code.as_str(), status) {
        ("NoSuchKey", _) => Error::NoSuchKey {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        ("NoSuchBucket", _) => Error::NoSuchBucket {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        ("AccessDenied", _) => Error::AccessDenied {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        _ if status >= 500 => Error::Server {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
        _ => Error::Client {
            code: parsed.code,
            message: parsed.message,
            status,
            request_id: parsed.request_id,
            host_id: parsed.host_id,
        },
    }
}
