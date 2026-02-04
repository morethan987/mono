use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::error::{MonoError, Result};
use crate::protocol::{Request, Response, decode_response, encode_request};

pub struct DaemonClient {
    stream: UnixStream,
    timeout: Duration,
}

impl DaemonClient {
    pub async fn connect(socket_path: &Path, timeout_secs: u64) -> Result<Self> {
        let timeout = Duration::from_secs(timeout_secs);

        let stream = tokio::time::timeout(timeout, UnixStream::connect(socket_path))
            .await
            .map_err(|_| MonoError::IpcTimeout { timeout_secs })?
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound
                    || e.kind() == std::io::ErrorKind::ConnectionRefused
                {
                    MonoError::DaemonNotRunning
                } else {
                    MonoError::IpcConnection(e)
                }
            })?;

        Ok(Self { stream, timeout })
    }

    pub async fn request(&mut self, request: Request) -> Result<Response> {
        let encoded = encode_request(&request)?;

        tokio::time::timeout(self.timeout, self.stream.write_all(encoded.as_bytes()))
            .await
            .map_err(|_| MonoError::IpcTimeout {
                timeout_secs: self.timeout.as_secs(),
            })?
            .map_err(MonoError::IpcSend)?;

        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();

        tokio::time::timeout(self.timeout, reader.read_line(&mut line))
            .await
            .map_err(|_| MonoError::IpcTimeout {
                timeout_secs: self.timeout.as_secs(),
            })?
            .map_err(MonoError::IpcReceive)?;

        decode_response(&line)
    }
}
