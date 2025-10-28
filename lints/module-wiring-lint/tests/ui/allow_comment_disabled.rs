#![feature(register_tool)]
#![register_tool(module_wiring)]
#![warn(module_wiring::no_parent_reexport)]

mod child {
  pub struct Thing;
  pub struct Other;
}

// allow module_wiring::no_parent_reexport
pub use self::child::Thing;

pub use self::child::Other; // allow module_wiring::no_parent_reexport

fn main() {}
