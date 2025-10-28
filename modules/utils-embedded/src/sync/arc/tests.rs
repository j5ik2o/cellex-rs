#![allow(clippy::disallowed_types)]
use cellex_utils_core_rs::sync::StateCell;

use super::*;
use crate::tests::init_arc_critical_section;

fn prepare() {
  init_arc_critical_section();
}

#[test]
fn arc_state_cell_updates() {
  prepare();
  let cell = ArcLocalStateCell::new(0_u32);
  let cloned = cell.clone();

  {
    let mut value = cloned.borrow_mut();
    *value = 9;
  }

  assert_eq!(*cell.borrow(), 9);
}

#[test]
fn arc_shared_try_unwrap() {
  prepare();
  let shared = ArcShared::new(7_u32);
  assert_eq!(ArcShared::new(3_u32).try_unwrap().unwrap(), 3);
  let clone = shared.clone();
  assert!(clone.try_unwrap().is_err());
}

#[test]
fn arc_state_cell_into_arc_exposes_inner() {
  prepare();
  let cell = ArcLocalStateCell::new(5_u32);
  let arc = cell.clone().into_arc();
  assert_eq!(*arc.try_lock().unwrap(), 5);
}
