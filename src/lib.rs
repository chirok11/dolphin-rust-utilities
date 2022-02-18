#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

#[macro_use]
extern crate log;

mod proxy;
mod zip;

#[cfg(target_os = "windows")]
mod addon_windows;

#[cfg(target_os = "macos")]
mod addon_darwin;

#[cfg(not(target_os = "windows"))]
mod addon_unix;

#[napi]
fn rust_logger_init() {
  pretty_env_logger::init();
}