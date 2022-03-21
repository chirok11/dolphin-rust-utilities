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
  let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(format!("{}:{}", ip, port)))
      .await
      .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))??;
  debug!("connected");
  // write request
  let auth_header = match username {
    Some(v) => format!("Proxy-Authorization: Basic {}\r\n", base64::encode(format!("{}:{}", v, password.unwrap()))),
    None => "".to_string()
  };
  let request = format!("GET http://vo4.co/ip-info HTTP/1.1\r\nHost: vo4.co\r\n{}User-Agent: DolphinProxy/7.81.0\r\nAccept: */*\r\n\r\n", auth_header);
  let r = stream.write(request.as_bytes()).await?;
  debug!("written {}", r);

  let reader = BufReader::new(stream);
  let result = read_stream(reader).await;

  match result {
    Some(body) => Ok(body),
    None => Err(napi::Error::new(napi::Status::GenericFailure, "Unable to read response".to_string()))
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
  .await.map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))??;
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
    if username.is_none() || password.is_none() {
      return Err(napi::Error::new(napi::Status::GenericFailure, "Auth is required, but no username/password provided".to_string()));
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
    let _ = stream.read(&mut buf).await?;
  }

  if (buf[0] == 0x01 || buf[0] == 0x05) && buf[1] == 0x00 {
    debug!("auth success");
    let mut wl: Vec<u8> = Vec::new();
    wl.extend(&[0x05, 0x01, 0x00, 0x03, 0x06]);
    wl.extend("vo4.co".as_bytes());
    wl.extend(&[0x00, 0x50]);
    debug!("requesting endpoint");
    let r = stream.write(&wl).await?;
    assert_eq!(r, wl.len());
  } else {
    return Err(napi::Error::new(napi::Status::GenericFailure, "Invalid username or password provided".to_string()));
  }

  let _ = stream.read(&mut buf).await?;

  if buf[1] == 0x00 {
    debug!("connection ok; write request");
    let req =
        "GET /ip-info HTTP/1.1\r\nHost: vo4.co\r\nUser-Agent: DolphinProxy/1.0\r\n\r\n".as_bytes();
    let w = stream.write(req).await?;
    assert_eq!(w, req.len());

    let reader = BufReader::new(stream);
    let result = read_stream(reader).await;

    return match result {
      Some(body) => Ok(body),
      None => Err(napi::Error::new(napi::Status::GenericFailure, "Unable to read response".to_string()))
    }
  } else {
    println!("connection failed: {:?}", &buf[0..4]);
  }

  Ok("".to_string())
}

async fn read_stream(mut buffer: BufReader<TcpStream>) -> Option<String> {
  let mut data = String::new();
  loop {
    let ok = buffer.read_line(&mut data).await;
    match ok {
      Ok(n) if n == 2 || n == 0 => break,
      Ok(n) => { debug!("read {}", &n) },
      Err(e) => { error!("{:#?}", e) }
    }
  }

  info!("{:?}", data);

  let mut length = None;
  let mut chunked = false;
  let mut body = vec![];

  if data.contains("chunked") {
    chunked = true;
    loop {
      buffer.read_until(b'\n', &mut body).await.unwrap();
      if body.ends_with(&[13, 10, 48, 13, 10, 13, 10]) { break; }
      debug!("{:?}", data);
    }
  } else if data.contains("Content-Length") {
    // content-length provided
    let headers = data.split("\r\n").collect::<Vec<&str>>();
    for header in headers {
      let mut x = header.split(": ");
      let (name, value) = (x.next(), x.next());
      debug!("{:?}: {:?}", name, value);
      if let Some(name) = name {
        if name == "Content-Length" {
          // parse int
          length = value.unwrap().parse::<usize>().ok();
        }
      }
    }

    if let Some(length) = length {
      let mut buf = [0; 4096];
      buffer.read_exact(&mut buf[..length]).await.unwrap();
      body.extend_from_slice(&buf[..length]);
      debug!("read: {}; {:?}", length, &buf);
    }
  }

  let lossy = String::from_utf8_lossy(&body);
  let body = lossy.split('\r').collect::<Vec<&str>>();

  debug!("body len: {}", body.len());
  debug!("{:?}", body);

  let body = body.iter().map(|v| match v.starts_with('\n') && v.len() < 5 {
    true => "",
    false => v
  }).collect::<Vec<&str>>();

  if chunked && body.len() > 1 {
    Some(body[1..].join(""))
  } else if length.is_some() {
    Some(body[0].to_string())
  } else {
    None
  }
}