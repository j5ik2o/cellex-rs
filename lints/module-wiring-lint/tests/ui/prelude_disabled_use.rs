#![feature(register_tool)]
#![register_tool(module_wiring)]
#![warn(module_wiring::no_parent_reexport)]

mod prelude {
  pub struct X;
}

pub use prelude::X;

fn main() {}
