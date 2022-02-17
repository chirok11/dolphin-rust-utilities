#![deny(clippy::all)]

use std::fmt::format;
use std::fs::read;
use std::time::Duration;
use napi::Result;
use napi::Status::GenericFailure;
use reqwest::Response;
use tokio::io::*;
use tokio::net::TcpStream;
use tokio::time::{Timeout, timeout};

#[macro_use]
extern crate napi_derive;

#[macro_use]
extern crate log;

#[napi]
fn sum(a: i32, b: i32) -> i32 {
  a + b
}

#[napi]
fn logger_init() {
  pretty_env_logger::init();
}

#[napi]
async fn proxy_check_http(
  ip: String,
  port: u32,
  username: Option<String>,
  password: Option<String>,
) -> Result<String> {

  // reqwest variant
  let proxy = reqwest::Proxy::http(format!("http://{}:{}", ip, port)).unwrap().basic_auth(&*username.unwrap(), &*password.unwrap());
  let client = reqwest::Client::builder().proxy(proxy).timeout(Duration::from_secs(5)).build().unwrap();
  let res = client.get("http://vo4.co/ip-info").send().await;

  match res {
    Ok(response) => Ok(response.text().await.unwrap()),
    Err(error) => Err(napi::Error::new(GenericFailure, format!("{}", error)))
  }

  debug!("connecting to {}:{}", ip, port);
  let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(format!("{}:{}", ip, port)))
      .await
      .map_err(|e| napi::Error::new(GenericFailure, e.to_string()))??;
  debug!("connected");
  // write request
  let auth_header = match username {
    Some(v) => format!("Proxy-Authorization: Basic {}\r\n", base64::encode(format!("{}:{}", v, password.unwrap()))),
    None => "".to_string()
  };
  let request = format!("GET http://vo4.co/ip-info HTTP/1.1\r\nHost: vo4.co\r\n{}User-Agent: DolphinProxy/7.81.0\r\nAccept: */*\r\n\r\n", auth_header);
  let r = stream.write(request.as_bytes()).await?;
  debug!("written {}", r);

  let mut buf = [0; 1024];
  let mut data = vec![];

  loop {
    let n = match stream.read(&mut buf).await {
      Ok(n) if n == 0 => { debug!("end of buffer"); break },
      Ok(n) => {
        data.write_all(&buf[0..n]).await?;
        n
      }
      Err(e) => {
        error!("{:#?}", e);
        break;
      }
    };
    debug!("read {}", n);
    debug!("last 8 bytes {:#?}", &data[data.len()-8..]);
    if data[data.len()-4..] == [13, 10, 13, 10] { debug!("rnrn found"); break; }
  }

  let lossy = String::from_utf8_lossy(&data);
  debug!("{}", lossy);
  let str: Vec<&str> = lossy.split('\r').filter(|p| p.contains('{')).collect();
  if !str.is_empty() {
    Ok(str[0].to_string())
  } else {
    debug!("{}", lossy);
    Err(napi::Error::new(GenericFailure, "Unable to read response".to_string()))
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
  let mut stream = timeout(Duration::from_secs(5),
  TcpStream::connect(format!("{}:{}", ip, port)))
  .await.map_err(|e| napi::Error::new(GenericFailure, e.to_string()))??;
  debug!("connected.");
  let mut buf = [0; 128];

  // write hello
  debug!("hello write start");
  let r = stream.write(&[5, 3, 0, 1, 2]).await?;
  debug!("write: {}", r);
  assert_eq!(r, 5);
  debug!("hello read start");
  let r = stream.read(&mut buf).await?;
  debug!("hello read: {}", r);

  if buf[0] == 0x05 && buf[1] == 0x02 {
    debug!("should auth");
    if (username.is_none() || password.is_none()) {
      return Err(napi::Error::new(GenericFailure, "Auth is required, but no username/password provided".to_string()));
    }
    // should auth
    let mut wl: Vec<u8> = Vec::new();
    let username = username.unwrap();
    let password = password.unwrap();
    wl.extend(&[0x01]);
    wl.push(username.len() as u8);
    wl.extend(username.as_bytes());
    wl.push(password.len() as u8);
    wl.extend(password.as_bytes());
    let r = stream.write(&wl).await?;
    assert_eq!(r, wl.len());
  }
  let _ = stream.read(&mut buf).await?;

  if buf[0] == 0x01 && buf[1] == 0x00 {
    debug!("auth success");
    let mut wl: Vec<u8> = Vec::new();
    wl.extend(&[5, 1, 0, 1]);
    wl.extend(&[0x05, 0x16, 0xd0, 0x87, 0x00, 0x50]);
    debug!("requesting endpoint");
    let r = stream.write(&wl).await?;
    assert_eq!(r, wl.len());
  } else {
    return Err(napi::Error::new(GenericFailure, "Invalid username or password provided".to_string()));
  }

  let _ = stream.read(&mut buf).await?;

  if buf[3] == 0x01 {
    debug!("connection ok; write request");
    let req =
        "GET /ip-info HTTP/1.1\r\nHost: vo4.co\r\nUser-Agent: DolphinProxy/1.0\r\n\r\n".as_bytes();
    let w = stream.write(req).await?;
    assert_eq!(w, req.len());

    let mut buf = [0; 1024];
    let mut data = vec![];

    loop {
      let n = match stream.read(&mut buf).await {
        Ok(n) if n == 0 => break,
        Ok(n) => {
          data.write_all(&buf[0..n]).await?;
          n
        }
        Err(e) => {
          error!("{:#?}", e);
          break;
        }
      };
      if data[data.len()-2..] == [13, 10] { break; }
      debug!("read {}", n);
      debug!("last two bytes {:#?}", &data[data.len()-2..]);
    }

    let lossy = String::from_utf8_lossy(&data);
    debug!("{}", lossy);
    let str: Vec<&str> = lossy.split('\r').filter(|p| p.contains('{')).collect();
    if !str.is_empty() {
      return Ok(str[0].to_string());
    } else {
      debug!("{}", lossy);
      return Err(napi::Error::new(GenericFailure, "Unable to read response".to_string()));
    }
  } else {
    println!("connection failed: {:?}", &buf[0..4]);
  }

  Ok("".to_string())
}
