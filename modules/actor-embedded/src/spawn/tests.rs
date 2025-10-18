extern crate std;

use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};

use super::*;

#[test]
fn immediate_spawner_drops_future_without_polling() {
  let spawner = ImmediateSpawner;
  let polled = Arc::new(AtomicBool::new(false));
  let flag = polled.clone();

  spawner.spawn(async move {
    flag.store(true, Ordering::SeqCst);
  });

  assert!(!polled.load(Ordering::SeqCst), "future should not be polled by ImmediateSpawner");
}
