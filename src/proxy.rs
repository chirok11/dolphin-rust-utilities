use napi::Result;
use reqwest::header::HeaderName;
use std::time::Duration;

#[allow(unused)]
#[napi]
async fn proxy_check_http(
  ip: String,
  port: u32,
  username: Option<String>,
  password: Option<String>,
) -> Result<String> {
  debug!("connecting to {}:{}", ip, port);
  let proxy = reqwest::Proxy::http(format!("{}:{}", ip, port)).unwrap();
  let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(30))
    .proxy(proxy)
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.4844.82 Safari/537.36")
    .use_rustls_tls()
    .http1_title_case_headers()
    .build()
    .unwrap();

  let request = match username {
    Some(username) => client
      .get("http://vo4.co/ip-info")
      .header("Accept", "text/plain")
      .header(
        HeaderName::from_bytes(b"Proxy-Authorization").unwrap(),
        format!(
          "Basic {}",
          base64::encode(&format!("{}:{}", username, password.unwrap()))
        ),
      ),
    None => client.get("http://vo4.co/ip-info"),
  };

  let request = request.build().unwrap();
  let result = client.execute(request).await;
  match result {
    Ok(response) => Ok(response.text().await.unwrap()),
    Err(e) => Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!("{}", &e),
    )),
  }
}

#[allow(unused)]
#[napi]
async fn proxy_check_socks5h(
  ip: String,
  port: u32,
  username: Option<String>,
  password: Option<String>,
) -> Result<String> {
  debug!("connecting to {}:{}", ip, port);

  let mut proxy = reqwest::Proxy::all(format!("socks5h://{}:{}", ip, port)).unwrap();

  if let Some(u) = username {
    proxy = proxy.basic_auth(&u, &password.unwrap());
  }

  let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(30))
    .proxy(proxy)
    .use_rustls_tls()
    .danger_accept_invalid_certs(true)
    .http1_title_case_headers()
    .build()
    .unwrap();

  let request = client.get("https://vo4.co/ip-info").build().unwrap();
  let result = client.execute(request).await;

  match result {
    Ok(response) => Ok(response.text().await.unwrap()),
    Err(e) => Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!("{}", &e),
    )),
  }
}

#[allow(unused)]
#[napi]
async fn proxy_check_socks5(
  ip: String,
  port: u32,
  username: Option<String>,
  password: Option<String>,
) -> Result<String> {
  debug!("connecting to {}:{}", ip, port);

  let mut proxy = reqwest::Proxy::all(format!("socks5://{}:{}", ip, port)).unwrap();

  if let Some(u) = username {
    proxy = proxy.basic_auth(&u, &password.unwrap());
  }

  let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(30))
    .proxy(proxy)
    .http1_title_case_headers()
    .danger_accept_invalid_certs(true)
    .build()
    .unwrap();

  let request = client.get("https://vo4.co/ip-info").build().unwrap();
  let result = client.execute(request).await;

  match result {
    Ok(response) => Ok(response.text().await.unwrap()),
    Err(e) => Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!("{}", &e),
    )),
  }
}
