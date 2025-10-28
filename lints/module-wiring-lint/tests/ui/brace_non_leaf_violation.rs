#![feature(register_tool)]
#![register_tool(module_wiring)]
#![warn(module_wiring::no_parent_reexport)]

mod parent {
  pub struct A;

  mod helpers {
    pub struct Hidden;
  }
}

pub use parent::{A};

fn main() {}
