#![feature(register_tool)]
#![register_tool(module_wiring)]
#![warn(module_wiring::no_parent_reexport)]

mod child {
  pub struct A;
}

pub use child::{A as Alias};

fn main() {}
