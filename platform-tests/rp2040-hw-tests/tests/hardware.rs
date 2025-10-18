#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use defmt_test::tests;
use panic_probe as _;
#[tests]
mod hardware {
  use super::*;

  #[test]
  fn smoke_led_placeholder() {
    info!("RP2040 hardware smoke test placeholder");
    defmt::assert!(true);
  }
}
