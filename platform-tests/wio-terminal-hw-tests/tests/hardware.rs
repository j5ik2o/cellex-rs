#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use defmt_test::tests;
#[tests]
mod hardware {
  use super::*;

  #[test]
  fn smoke_led_placeholder() {
    info!("Wio Terminal hardware smoke test placeholder");
    defmt::assert!(true);
  }
}
