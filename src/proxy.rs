use std::time::Duration;
use napi::Result;
use tokio::{time::timeout, net::TcpStream, io::{AsyncWriteExt, BufReader, AsyncReadExt, AsyncBufReadExt}};


#[napi]
async fn proxy_check_http(
  ip: String,
  port: u32,
  username: Option<String>,
  password: Option<String>,
) -> Result<String> {
  debug!("connecting to {}:{}", ip, port);
  let mut proxy = reqwest::Proxy::http(format!("{}:{}", ip, port)).unwrap();
  if let Some(u) = username {
    proxy = proxy.basic_auth(&u, &password.unwrap())
  }
  let client = reqwest::Client::builder().connect_timeout(Duration::from_secs(30)).proxy(proxy).build().unwrap();
  let request = client.get("http://vo4.co/ip-info").build().unwrap();
  let result = client.execute(request).await;
  match result {
    Ok(response) => Ok(response.text().await.unwrap()),
    Err(e) => Err(napi::Error::new(napi::Status::GenericFailure, format!("{}", &e)))
  }
}

#[napi]
async fn proxy_check_socks5(
  ip: String,
  port: u32,
  username: Option<String>,
  password: Option<String>,
) -> Result<String> {
  debug!("connecting to {}:{}", ip, port);
  
  let mut proxy = reqwest::Proxy::http(format!("socks5://{}:{}", ip, port)).unwrap();

  if let Some(u) = username {
    proxy = proxy.basic_auth(&u, &password.unwrap());
  }

  let client = reqwest::Client::builder().connect_timeout(Duration::from_secs(30)).proxy(proxy).build().unwrap();
  let request = client.get("http://vo4.co/ip-info").build().unwrap();
  let result = client.execute(request).await;
  match result {
    Ok(response) => Ok(response.text().await.unwrap()),
    Err(e) => Err(napi::Error::new(napi::Status::GenericFailure, format!("{}", &e)))
  }
}