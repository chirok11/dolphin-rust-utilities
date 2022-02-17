#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

#[macro_use]
extern crate log;

mod proxy;
mod zip;

#[cfg(target_os = "windows")]
mod addon_windows;

#[napi]
fn logger_init() {
  pretty_env_logger::init();
}