use std::path::Path;

use rusqlite::Connection;

#[napi(object)]
#[derive(Debug)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

#[napi]
async fn sqlite_add_login_password(
  filepath: String,
  username: String,
  password: String,
) -> napi::Result<LoginData> {
  let db_path = Path::new(&filepath);
  if let Err(e) = tokio::fs::metadata(db_path).await {
    error!("File does not exists {}", e);
    return Err(napi::Error::from_reason("File does not exists".to_string()));
  }

  let connection = Connection::open(db_path);
  if let Ok(connection) = connection {
    // Before we check do we have login for coinlist.co
    let mut query = match connection.prepare("SELECT username_value, password_value FROM logins WHERE username_element = 'user[email]' AND origin_url LIKE '%coinlist.co%' LIMIT 1") {
        Ok(query) => query,
        Err(e) => {
            error!("Error while preparing query: {}", e);
            return Err(napi::Error::from_reason(format!("Error while preparing query: {}", e)));
        }
    };

    let rows = query.query([]);

    match rows {
      Ok(mut rows) => {
        if let Ok(row) = rows.next() {
          if let Some(row) = row {
            let username_value = row.get::<_, String>(0);
            let password_value = row.get::<_, Vec<u8>>(1);

            if username_value.is_ok() && password_value.is_ok() {
              let u = username_value.unwrap();
              let p = password_value.unwrap();
              let putf = String::from_utf8(p).map_err(|e| napi::Error::from_reason(e.to_string()))?;

              debug!("Found login for coinlist.co: {} {}", u, putf);
              return if u == username && putf == password {
                debug!("Login matches, skipping");
                Ok(LoginData {
                  username,
                  password
                })
              } else {
                // We have login for coinlist.co but it is not correct
                // We need to update it
                debug!("Login does not match, updating");
                return Ok(
                  LoginData {
                    username: u,
                    password: putf
                  }
                );
                // match connection.prepare("UPDATE logins SET username_value = ?, password_value = ? WHERE username_element = 'user[email]' AND origin_url LIKE '%coinlist.co%'") {
                //   Ok(mut statement) => {
                //     let result = statement.execute([&username, &password]);
                //     if result.is_err() {
                //       let e = result.unwrap_err();
                //       error!("Error while updating login: {}", e);
                //       Err(napi::Error::from_reason(format!("{:?}", e)))
                //     } else {
                //       Ok(LoginData {
                //         username,
                //         password
                //       })
                //     }
                //   },
                //   Err(e) => {
                //     error!("Error while preparing query: {}", e);
                //     Err(napi::Error::from_reason(format!("{}", e)))
                //   }
                // }
              }
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
            id, \
            date_last_used, \
            moving_blocked_for, \
            date_password_modified\
            ) VALUES ('https://coinlist.co/login', 'https://coinlist.co/login', 'user[email]', ?, 'user[password]', ?, '', 'https://coinlist.co/', '13298985082883522', 0, 0, 0, 0, '', '', '', '',\
             0, 0, '', 0, '13298985082883522', '', '')";
            let mut statement = connection.prepare(query).map_err(|e| napi::Error::from_reason(e.to_string()))?;
            let result = statement.execute([&username, &password]);

            return match result {
              Ok(_len) => {
                debug!("Inserted new login for coinlist.co: {} {}", username, password);
                Ok(LoginData {
                  username,
                  password
                })
              },
              Err(e) => {
                error!("Error while inserting new login: {}", e);
                Err(napi::Error::from_reason(format!("{}", e)))
              }
            }
          }
        } else {

        }
      }
      Err(e) => {
        error!("Unable to query database: {}", e);
        return Err(napi::Error::from_reason(format!("Unable to query database: {}", e)));
      }
    }
  } else {
    let e = connection.err();
    error!("Unable to open database, error: {:?}", e);
    return Err(napi::Error::from_reason(format!("Unable to open database, error: {:?}", e)));
  }

  Ok(LoginData {
    username,
    password
  })
}

#[tokio::test]
async fn test_sqlite_add_login_password() {
  pretty_env_logger::init();

  let filepath = "/home/pq/dolphin-profile/Default/Login Data";
  let result = sqlite_add_login_password(
    filepath.to_string(),
    "username@paxssword.com".to_string(),
    "abcdef123".to_string(),
  )
  .await;

  println!("{:?}", result);
}
