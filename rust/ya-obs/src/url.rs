use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use url::Url;

use crate::config::{AddressingStyle, ClientConfig};
use crate::error::Error;

const KEY_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'?')
    .add(b'`').add(b'{').add(b'}').add(b'%').add(b'+');

fn encode_key(key: &str) -> String {
    key.split('/')
        .map(|seg| utf8_percent_encode(seg, KEY_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_dns_safe_bucket(bucket: &str) -> bool {
    !bucket.is_empty() && !bucket.contains('.') && bucket.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn base_host(cfg: &ClientConfig) -> Result<String, Error> {
    if let Some(ep) = &cfg.endpoint {
        let u = Url::parse(ep).map_err(|e| Error::Config(format!("invalid endpoint {ep}: {e}")))?;
        Ok(u.host_str()
            .ok_or_else(|| Error::Config(format!("endpoint missing host: {ep}")))?
            .to_string())
    } else if let Some(region) = &cfg.region {
        Ok(format!("obs.{region}.myhuaweicloud.com"))
    } else {
        Err(Error::Config("either region or endpoint must be set".into()))
    }
}

pub fn build_object_url(cfg: &ClientConfig, bucket: &str, key: &str) -> Result<Url, Error> {
    let host = base_host(cfg)?;
    let style = match cfg.addressing_style {
        AddressingStyle::Auto => {
            if is_dns_safe_bucket(bucket) { AddressingStyle::Virtual } else { AddressingStyle::Path }
        }
        s => s,
    };

    let url_str = match style {
        AddressingStyle::Virtual => format!("https://{bucket}.{host}/{}", encode_key(key)),
        AddressingStyle::Path => format!("https://{host}/{bucket}/{}", encode_key(key)),
        AddressingStyle::Auto => unreachable!(),
    };
    Url::parse(&url_str).map_err(|e| Error::Config(format!("built invalid url: {e}")))
}
