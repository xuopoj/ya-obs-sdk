use time::{macros::format_description, OffsetDateTime};

use crate::config::{ClientConfig, SigningVersion};
use crate::error::Error;
use crate::signer::{v2, v4};
use crate::url::build_object_url;

const AMZ_DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
const DATE: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]");

pub fn presign_get_object(
    cfg: &ClientConfig,
    bucket: &str,
    key: &str,
    expires: u64,
) -> Result<String, Error> {
    let creds = cfg.credentials.as_ref()
        .ok_or_else(|| Error::Config("credentials required for presign".into()))?;
    let url = build_object_url(cfg, bucket, key)?;
    let now = OffsetDateTime::now_utc();

    match cfg.signing_version {
        SigningVersion::V4 => {
            let region = cfg.region.as_deref()
                .ok_or_else(|| Error::Config("region required for V4 presign".into()))?;
            let amz_date = now.format(AMZ_DATE).unwrap();
            let date = now.format(DATE).unwrap();
            Ok(v4::presign_url(
                "GET", url.as_str(),
                &creds.access_key, &creds.secret_key,
                &amz_date, &date, region, "s3",
                expires,
            ))
        }
        SigningVersion::V2 => {
            let expires_unix = now.unix_timestamp() as u64 + expires;
            let (signed, _sts) = v2::presign_url(
                "GET", url.as_str(),
                bucket, key, expires_unix,
                &std::collections::BTreeMap::new(),
                &creds.access_key, &creds.secret_key,
            );
            Ok(signed)
        }
    }
}
