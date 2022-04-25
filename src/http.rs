use bytes::{Buf, BytesMut};
use futures_util::{StreamExt, TryStreamExt};
use napi::threadsafe_function::{
  ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};

use napi::JsFunction;
use napi::Status::GenericFailure;
use reqwest::Request;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::io::{Cursor, Read, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter, ReadBuf};
use tokio::time::sleep;

#[napi]
struct HttpFileDownloader {
  emitter: Option<ThreadsafeFunction<DownloadProgress, ErrorStrategy::Fatal>>,
}

#[napi(object)]
#[derive(Serialize, Deserialize, Clone)]
struct DownloadProgress {
  pub target: &'static str,
  pub downloaded: i64,
  pub total: Option<i64>,
}

pub trait ReqwestExt<T> {
  fn napify(self) -> napi::Result<T>;
}

impl ReqwestExt<reqwest::Response> for std::result::Result<reqwest::Response, reqwest::Error> {
  fn napify(self) -> napi::Result<reqwest::Response> {
    match self {
      Ok(t)
        if matches!(
          t.status(),
          reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::FORBIDDEN
        ) =>
      {
        Err(napi::Error::from_reason(format!(
          "Response status code is invalid: {:?}",
          t.status()
        )))
      }
      Ok(t) => Ok(t),
      Err(e) => Err(napi::Error::from_reason(format!("{}", e))),
    }
  }
}

pub enum ECode {
  ContentLengthMatchFileSize = 0,
  ContentLengthIsNotSupported = 1,
  ChecksumVerificationFailed = 2,
  Unknown = 3,
}

#[napi(object)]
pub struct HttpFileDownloaderResponse {
  pub status: bool,
  pub ecode: u32,
  pub message: &'static str,
}

#[napi]
impl HttpFileDownloader {
  #[napi(constructor)]
  pub fn new(emitter: Option<JsFunction>) -> Self {
    debug!("new: {}", emitter.is_some());
    if let Some(func) = emitter {
      let tsfn: ThreadsafeFunction<_, ErrorStrategy::Fatal> = func
        .create_threadsafe_function(0, |ctx: ThreadSafeCallContext<DownloadProgress>| {
          Ok(vec![
            ctx.env.create_string(ctx.value.target)?.into_unknown(),
            ctx.env.to_js_value(&ctx.value)?.into_unknown(),
          ])
        })
        .unwrap();
      Self {
        emitter: Some(tsfn),
      }
    } else {
      Self { emitter: None }
    }
  }

  #[napi]
  pub async fn download_file(
    &mut self,
    url: String,
    file: String,
  ) -> napi::Result<HttpFileDownloaderResponse> {
    let client = reqwest::Client::builder()
      .user_agent("value")
      .http1_title_case_headers()
      .use_rustls_tls()
      .build()
      .map_err(|e| napi::Error::from_reason(format!("{}", e)))?;
    let path = PathBuf::from(file);
    let file_length = match fs::metadata(&path) {
      Ok(t) => t.len(),
      Err(_) => 0,
    };
    let response = client.get(&url).send().await.napify().unwrap();
    // Check if headers present Accept-Ranges and Content-Length
    let ranges = response.headers().contains_key("Accept-Ranges");
    info!("[HTTP] Header Accept-Ranges: {}", ranges);
    info!("[File] {} file length: {}", path.display(), file_length);

    let content_length = match response.content_length() {
      Some(cl) => match file_length.cmp(&cl) {
        Ordering::Less => cl,
        Ordering::Equal => {
          return Ok(HttpFileDownloaderResponse {
            status: true,
            ecode: ECode::ContentLengthMatchFileSize as u32,
            message: "File size equals content-length",
          })
        }
        Ordering::Greater => {
          return Ok(HttpFileDownloaderResponse {
            status: false,
            ecode: ECode::ChecksumVerificationFailed as u32,
            message: "Unable to verify checksum. File on server is changed",
          })
        }
      },
      None => {
        return Ok(HttpFileDownloaderResponse {
          status: false,
          ecode: ECode::ContentLengthIsNotSupported as u32,
          message: "Server does not support Content-Length",
        })
      }
    };
    info!("[HTTP] Content-Length: {}", content_length);
    let checksum = if file_length > 65535 { 65535 } else { 0 };
    let mut request = client.get(&url);
    if ranges && checksum > 0 {
      request = request.header(
        "Range",
        format!("bytes={}-{}", file_length - checksum, content_length),
      );
    }

    let response = request.send().await.napify()?;
    let mut file = OpenOptions::new()
      .append(true)
      .read(true)
      .write(true)
      .create(true)
      .truncate(!ranges)
      .open(&path)
      .await?;
    let mut checksum_buf = vec![0; 65535];

    let mut stream = response.bytes_stream();
    // Go to end of file and read last 65535 bytes
    if ranges && checksum > 0 {
      debug!("[File] Reading last 65535 bytes");
      file.seek(SeekFrom::End(-65535)).await?;
      let n = file.read_exact(&mut checksum_buf).await?;
      debug!("[File] Read {} bytes", n);
      debug!("[File] Checksum: {:?}", checksum_buf.len());

      let mut buffer: Vec<u8> = Vec::new();
      while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| napi::Error::from_reason(format!("{}", e)))?;
        let bytes = chunk.as_ref().to_vec();
        // on_chunk(bytes.len(), content_length);
        buffer.extend(bytes);
        if buffer.len() > 65535 && checksum_buf[..] != buffer[..65535] {
          return Ok(HttpFileDownloaderResponse {
            status: false,
            ecode: ECode::ChecksumVerificationFailed as u32,
            message: "Unable to verify checksum. File on server is changed",
          });
        } else if buffer.len() > 65535 && checksum_buf[..] == buffer[..65535] {
          let n = file.write(&buffer[65535..]).await?;
          debug!("[File] Written {} bytes", n);
          break;
        }
      }
      debug!("[HTTP] Downloaded {} bytes", buffer.len());
    }

    let mut n = file_length as usize;
    let emit = self.emitter.clone();

    while let Some(chunk) = stream.next().await {
      let chunk = chunk.map_err(|e| napi::Error::from_reason(format!("{}", e)))?;
      let bytes = chunk.as_ref().to_vec();
      file.write_all(&bytes).await?;
      n += bytes.len();

      if let Some(emit) = &emit {
        emit.call(
          DownloadProgress {
            target: "progress",
            downloaded: n as i64,
            total: Some(content_length as i64),
          },
          ThreadsafeFunctionCallMode::NonBlocking,
        );
      }

      file.flush().await?;
    }

    Ok(HttpFileDownloaderResponse {
      status: true,
      ecode: ECode::Unknown as u32,
      message: "OK",
    })
  }
}
