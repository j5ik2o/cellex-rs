#![deny(cfg_std_test)]

#[cfg(any(feature = "std", feature = "alloc"))]
fn mixed_guard() {}

fn main() {}
