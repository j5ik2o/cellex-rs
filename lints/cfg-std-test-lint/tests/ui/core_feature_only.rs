#![deny(cfg_std_test)]

#[cfg(feature = "std")]
fn std_only() {}

fn main() {}
