#![feature(register_tool)]
#![register_tool(module_wiring)]
#![warn(module_wiring::no_parent_reexport)]
// rustc-env:MODULE_WIRING_ALLOW_PRELUDE=1

pub mod prelude {
  mod hidden {
    pub struct Thing;
  }

  pub use self::hidden::Thing;
}

fn main() {}
