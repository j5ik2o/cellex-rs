use super::*;

#[test]
fn arc_state_cell_updates() {
  let cell = ArcStateCell::new(0_u32);
  let cloned = cell.clone();

  {
    let mut value = cloned.borrow_mut();
    *value = 5;
  }

  assert_eq!(*cell.borrow(), 5);
}
