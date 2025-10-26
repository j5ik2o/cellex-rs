#![feature(register_tool)]
#![register_tool(module_wiring)]
#![warn(module_wiring::no_parent_reexport)]
// rustc-env:MODULE_WIRING_ALLOW_COMMENT=1

mod child {
  pub struct Thing;
}

// allow module_wiring::no_parent_reexport
pub use self::child::Thing;

fn main() {}
