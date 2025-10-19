#![allow(clippy::disallowed_types)]
use super::*;

#[test]
fn rc_state_cell_updates() {
  let cell = RcStateCell::new(1_u32);
  let cloned = cell.clone();

  {
    let mut value = cloned.borrow_mut();
    *value = 5;
  }

  assert_eq!(*cell.borrow(), 5);
}

#[test]
fn rc_shared_try_unwrap_behavior() {
  let shared = RcShared::new(10_u32);
  let clone = shared.clone();

  assert!(clone.try_unwrap().is_err());
  assert_eq!(RcShared::new(5_u32).try_unwrap().unwrap(), 5);
}

#[test]
fn rc_shared_conversion_round_trip() {
  let rc = Rc::new(3_u32);
  let shared = RcShared::from_rc(rc.clone());
  assert!(Rc::ptr_eq(&shared.clone().into_inner(), &rc));
}
