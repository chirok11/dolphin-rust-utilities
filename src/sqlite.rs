use std::{path::Path, str::FromStr};

use reqwest::Url;
use rusqlite::Connection;
use tokio::{fs, io::AsyncWriteExt};

#[napi(object)]
#[derive(Debug)]
pub struct LoginData {
  pub username: String,
  pub password: String,
}

#[napi(object)]
#[derive(Debug)]
pub struct LoginCreationParams {
  pub username: String,
  pub password: String,
  // url data
  pub url: String,
  pub username_field: String,
  pub password_field: String,
}

#[napi]
async fn sqlite_add_login_password(
  filepath: String,
  login_params: LoginCreationParams,
) -> napi::Result<LoginData> {
  let db_path = Path::new(&filepath);
  if let Err(e) = tokio::fs::metadata(db_path).await {
    error!("File does not exists {}", e);
    return Err(napi::Error::from_reason("File does not exists".to_string()));
  }

  let connection = Connection::open(db_path);
  if let Ok(connection) = connection {
    // Before we check do we have login for coinlist.co
    let mut query = match connection.prepare("SELECT username_value, password_value FROM logins WHERE username_element = ? AND origin_url = ? LIMIT 1") {
        Ok(query) => query,
        Err(e) => {
            error!("Error while preparing query: {}", e);
            return Err(napi::Error::from_reason(format!("Error while preparing query: {}", e)));
        }
    };

    let rows = query.query([&login_params.username_field, &login_params.url]);

    match rows {
      Ok(mut rows) => {
        if let Ok(row) = rows.next() {
          if let Some(row) = row {
            let username_value = row.get::<_, String>(0);
            let password_value = row.get::<_, Vec<u8>>(1);

            if username_value.is_ok() && password_value.is_ok() {
              let u = username_value.unwrap();
              let p = password_value.unwrap();
              let putf =
                String::from_utf8(p).map_err(|e| napi::Error::from_reason(e.to_string()))?;

              debug!("Found login for {}: {} {}", &login_params.url, u, putf);
              return if u == login_params.username && putf == login_params.password {
                debug!("Login matches, skipping");
                Ok(LoginData {
                  username: login_params.username,
                  password: login_params.password,
                })
              } else {
                debug!("Login does not match, will return username and password from database");
                return Ok(LoginData {
                  username: u,
                  password: putf,
                });
              };
            }
          } else {
            // Here we should insert new one with provided login, password
            // columns: origin_url, action_url, username_element, username_value, password_element, password_value, submit_element, signon_realm, date_created, blacklisted_by_user
            // scheme, password_type, times_used, form_data, display_name, icon_url, federation_url, skip_zero_click, generation_upload_status,
            // possible_username_pairs, id, date_last_used, moving_blocked_for, date_password_modified
            let query = "INSERT INTO logins (\
            origin_url, \
            action_url, \
            username_element, \
            username_value, \
            password_element, \
            password_value, \
            submit_element, \
            signon_realm, \
            date_created, \
            blacklisted_by_user, \
            scheme, \
            password_type, \
            times_used, \
            form_data, \
            display_name, \
            icon_url, \
            federation_url, \
            skip_zero_click, \
            generation_upload_status, \
            possible_username_pairs, \
            date_last_used, \
            moving_blocked_for, \
            date_password_modified\
            ) VALUES (?, ?, ?, ?, ?, ?, '', ?, '13298985082883522', 0, 0, 0, 0, '', '', '', '',\
             0, 0, '', '13298985082883522', '', '')";
            let url_parsed = Url::from_str(&login_params.url)
              .map_err(|e| napi::Error::from_reason(e.to_string()))?;

            let mut statement = connection
              .prepare(query)
              .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            let result = statement.execute([
              &login_params.url,
              &login_params.url,
              &login_params.username_field,
              &login_params.username,
              &login_params.password_field,
              &login_params.password,
              &format!(
                "{}://{}/",
                &url_parsed.scheme(),
                &url_parsed.host_str().unwrap()
              ),
            ]);

            return match result {
              Ok(_len) => {
                debug!(
                  "Inserted new login for coinlist.co: {} {}",
                  &login_params.username, &login_params.password
                );
                Ok(LoginData {
                  username: login_params.username,
                  password: login_params.password,
                })
              }
              Err(e) => {
                error!("Error while inserting new login: {}", e);
                Err(napi::Error::from_reason(format!("{}", e)))
              }
            };
          }
        } else {
        }
      }
      Err(e) => {
        error!("Unable to query database: {}", e);
        return Err(napi::Error::from_reason(format!(
          "Unable to query database: {}",
          e
        )));
      }
    }
  } else {
    let e = connection.err();
    error!("Unable to open database, error: {:?}", e);
    return Err(napi::Error::from_reason(format!(
      "Unable to open database, error: {:?}",
      e
    )));
  }

  Ok(LoginData {
    username: login_params.username,
    password: login_params.password,
  })
}

#[allow(unused)]
#[napi]
async fn create_sqlite_login_database(path: String) -> napi::Result<bool> {
  let mut file = fs::OpenOptions::new()
    .create(true)
    .append(false)
    .write(true)
    .open(path)
    .await?;
  let bytes = include_bytes!("../Login Data");
  file.write_all(bytes).await?;
  file.flush().await?;

  Ok(true)
}

#[tokio::test]
async fn test_sqlite_create_database() {
  let path = "/home/pq/database".into();
  let result = create_sqlite_login_database(path).await;

  println!("{:?}", result);
}

#[tokio::test]
async fn test_sqlite_add_coinlist_login_password() {
  pretty_env_logger::init();

  let filepath = "/Users/dark/chrome-profile/Default/Login Data";
  let params = LoginCreationParams {
    url: "https://coinlist.co/login".into(),
    username: "admin".into(),
    password: "kevin123".into(),
    username_field: "user[email]".into(),
    password_field: "user[password]".into(),
  };
  let result = sqlite_add_login_password(filepath.to_string(), params).await;

  println!("{:?}", result);
}

#[tokio::test]
async fn test_sqlite_add_facebook_login_password() {
  pretty_env_logger::init();

  let filepath = "/Users/dark/chrome-profile/Default/Login Data";
  let params = LoginCreationParams {
    url: "https://www.facebook.com/".into(),
    username: "test".into(),
    password: "test".into(),
    username_field: "email".into(),
    password_field: "pass".into(),
  };
  let result = sqlite_add_login_password(filepath.to_string(), params).await;

  println!("{:?}", result);
}

#[tokio::test]
async fn test_sqlite_add_business_facebook_login_password() {
  pretty_env_logger::init();

  let filepath = "/Users/dark/chrome-profile/Default/Login Data";
  let params = LoginCreationParams {
    url: "https://business.facebook.com/login/".into(),
    username: "test".into(),
    password: "test".into(),
    username_field: "email".into(),
    password_field: "pass".into(),
  };
  let result = sqlite_add_login_password(filepath.to_string(), params).await;

  println!("{:?}", result);
}

#[tokio::test]
async fn test_sqlite_add_ads_tiktok_login_password() {
  pretty_env_logger::init();

  let filepath = "/Users/dark/chrome-profile/Default/Login Data";
  let params = LoginCreationParams {
    url: "https://ads.tiktok.com/i18n/login/".into(),
    username: "test".into(),
    password: "test".into(),
    username_field: "email".into(),
    password_field: "password".into(),
  };
  let result = sqlite_add_login_password(filepath.to_string(), params).await;

  println!("{:?}", result);
}

#[tokio::test]
async fn test_sqlite_add_google_login_password() {
  pretty_env_logger::init();

  let filepath = "/Users/dark/chrome-profile/Default/Login Data";
  let params = LoginCreationParams {
    url: "https://accounts.google.com/signin/v2/identifier".into(),
    username: "abraham@gmail.com".into(),
    password: "abcdef123".into(),
    username_field: "identifier".into(),
    password_field: "password".into(),
  };
  let result = sqlite_add_login_password(filepath.to_string(), params).await;

  println!("{:?}", result);
}
