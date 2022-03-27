use bytes::BytesMut;
use futures_util::StreamExt;
use napi::threadsafe_function::{
  ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi::JsFunction;
use napi::Status::GenericFailure;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

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
    filename: String,
  ) -> napi::Result<HttpFileDownloaderResponse> {
    let client = reqwest::ClientBuilder::new()
      .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.82 Safari/537.36")
      .build()
      .map_err(|e| napi::Error::new(GenericFailure, format!("unable to build client: {}", &e)))?;

    let path = PathBuf::from(filename);
    //@ Get metadata for file if available.
    let file_length = match fs::metadata(&path) {
      Ok(metadata) => metadata.len(),
      Err(e) => {
        // maybe file does not exists.
        info!("[http] ignored error: {}", e);
        0
      }
    };

    debug!("[http] send GET request before we start");
    let first_request = client
      .get(url.clone())
      .build()
      .map_err(|e| napi::Error::new(GenericFailure, format!("{}", e)))?;
    let first_request_response = client.execute(first_request).await.napify()?;
    let bytes_supported = first_request_response
      .headers()
      .get("Accept-Ranges")
      .map(|v| v == "bytes")
      .unwrap();
    let content_length = match first_request_response.content_length() {
      Some(cl) => {
        if file_length == cl {
          return Ok(HttpFileDownloaderResponse {
            status: true,
            ecode: ECode::ContentLengthMatchFileSize as u32,
            message: "File size equals content-length",
          });
        } else {
          cl
        }
      }
      None => {
        return Ok(HttpFileDownloaderResponse {
          status: false,
          ecode: ECode::ContentLengthIsNotSupported as u32,
          message: "Server does not support Content-Length",
        })
      }
    };

    let checksum = if (file_length as f64 * 0.01) as u64 > 65535 {
      65553
    } else {
      (file_length as f64 * 0.01) as u64
    };

    let request = if bytes_supported {
      client
        .get(url.clone())
        .header(
          reqwest::header::RANGE,
          format!("bytes={}-{}", file_length - checksum, content_length),
        )
        .build()
    } else {
      client.get(url.clone()).build()
    }
    .map_err(|e| napi::Error::new(GenericFailure, format!("{}", e)))?;

    let response = client.execute(request).await.napify()?;
    let mut stream = response.bytes_stream();
    let mut file = OpenOptions::new()
      .create(true)
      .write(true)
      .append(true)
      .read(true)
      .truncate(!bytes_supported)
      .open(&path)
      .await?;

    // Before we should verify that contents is ok
    // do that only if checksum > 0
    if checksum > 0 {
      file.seek(SeekFrom::Start(file_length - checksum)).await?;

      let mut chkbuf = vec![0; checksum as usize];
      let n = file.read_exact(&mut chkbuf).await?;
      assert_eq!(checksum as usize, n);
      info!("read: {}", n);

      let mut b = BytesMut::new();
      while let Some(item) = stream.next().await {
        match item {
          Ok(chunk) => {
            b.extend_from_slice(&chunk);
            debug!("read to BytesMut: {}", chunk.len());
            if b.len() > n {
              debug!("b.len() > n");
              break;
            }
          }
          Err(e) => return Err(napi::Error::new(GenericFailure, format!("Error: {}", e))),
        }
      }

      debug!("file_length: {}", file_length);
      debug!("bytes={}-{}", file_length - checksum, content_length);
      debug!("b len: {}", b.len());
      debug!("chkbuf len: {}", chkbuf.len());
      let v1 = &chkbuf[..];
      let v3 = b.split_off(checksum as usize);
      let v2 = &b[..];

      debug!("validating bytes");
      if checksum > 0 && (v1.len() != v2.len() || v1 != v2) {
        debug!("[err] bytes mismatch");
        debug!(
          "v1({}) != v2({}) = {}",
          v1.len(),
          v2.len(),
          v1.len() != v2.len()
        );
        debug!("v1 != v2 = {}", v1 != v2);

        return Ok(HttpFileDownloaderResponse {
          status: false,
          ecode: ECode::ChecksumVerificationFailed as u32,
          message: "Unable to verify checksum. File on server is changed",
        });
      }

      // we should write remaining bytes
      if !v3.is_empty() {
        debug!("b.len() > checksum. should write remaining to file");
        file.write_all(&v3[..]).await?;
        file.flush().await?;
      }
    }

    debug!("[+] bytes check passed");

    let mut n = file_length as usize;
    let emit = self.emitter.clone();
    while let Some(chunk) = stream.next().await {
      match chunk {
        Ok(chunk) => {
          n += file.write(&chunk).await?;
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
        }
        Err(e) => return Err(napi::Error::new(GenericFailure, format!("Error: {}", &e))),
      }
    }

    debug!("download complete. flushing");
    file.flush().await?;

    if let Some(emit) = emit {
      emit.call(
        DownloadProgress {
          target: "progress",
          downloaded: n as i64,
          total: Some(content_length as i64),
        },
        ThreadsafeFunctionCallMode::NonBlocking,
      );
    }

    Ok(HttpFileDownloaderResponse {
      status: true,
      ecode: ECode::Unknown as u32,
      message: "OK",
    })
  }
}
