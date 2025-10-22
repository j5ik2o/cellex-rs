#![allow(cfg_std_test)]

#[cfg(feature = "std")]
fn uses_std() {}

use std::thread;

fn main() {
  thread::yield_now();
}
