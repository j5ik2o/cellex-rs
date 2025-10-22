#![deny(cfg_std_test)]

#[cfg(all(test, feature = "std"))]
fn uses_std() {}

fn main() {}
