#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

#[macro_use]
extern crate log;

mod proxy;

#[napi]
fn logger_init() {
  pretty_env_logger::init();
}