use bytes::Bytes;
use futures::StreamExt;

use crate::error::Error;

pub struct StreamingBody {
    inner: reqwest::Response,
}

impl StreamingBody {
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        Self { inner }
    }

    pub async fn read_to_end(self) -> Result<Vec<u8>, Error> {
        let bytes = self.inner.bytes().await.map_err(Error::Transport)?;
        Ok(bytes.to_vec())
    }

    pub fn into_stream(self) -> impl futures::Stream<Item = Result<Bytes, Error>> {
        self.inner
            .bytes_stream()
            .map(|r| r.map_err(Error::Transport))
    }
}
