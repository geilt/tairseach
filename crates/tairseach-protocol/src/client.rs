use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::{JsonRpcRequest, JsonRpcResponse};

pub struct SocketClient {
    reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    writer: tokio::io::WriteHalf<UnixStream>,
}

impl SocketClient {
    pub async fn connect() -> Result<Self, std::io::Error> {
        let Some(home) = dirs::home_dir() else {
            let err = std::io::Error::new(std::io::ErrorKind::NotFound, "home directory unavailable");
            eprintln!("tairseach-mcp: failed to locate home directory");
            return Err(err);
        };

        let path = home.join(".tairseach").join("tairseach.sock");
        Self::connect_to(path).await
    }

    pub async fn connect_to(path: PathBuf) -> Result<Self, std::io::Error> {
        match UnixStream::connect(&path).await {
            Ok(stream) => {
                let (r, w) = tokio::io::split(stream);
                Ok(Self {
                    reader: BufReader::new(r),
                    writer: w,
                })
            }
            Err(err) => {
                eprintln!(
                    "tairseach-mcp: could not connect to Tairseach socket at {} (is Tairseach running?): {}",
                    path.display(),
                    err
                );
                Err(err)
            }
        }
    }

    pub async fn call(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse, std::io::Error> {
        let payload = serde_json::to_string(&request)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        self.writer.write_all(payload.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line).await?;
        if bytes == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "socket closed while waiting for response",
            ));
        }

        let response: JsonRpcResponse = serde_json::from_str(line.trim())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(response)
    }
}
