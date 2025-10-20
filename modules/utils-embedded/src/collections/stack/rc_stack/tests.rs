#![allow(clippy::disallowed_types)]
use super::*;

#[test]
fn rc_stack_push_pop() {
  let stack = RcStack::with_capacity(1);
  stack.push(1).unwrap();
  assert!(stack.push(2).is_err());
  assert_eq!(stack.pop(), Some(1));
  assert!(stack.pop().is_none());
}

#[test]
fn rc_stack_handle_operations() {
  let stack = RcStack::new();
  stack.push(10).unwrap();
  let cloned = stack.clone();
  cloned.push(11).unwrap();

  assert_eq!(stack.len().to_usize(), 2);
  assert_eq!(cloned.pop(), Some(11));
  assert_eq!(stack.pop(), Some(10));
}

#[test]
fn rc_stack_peek_ref() {
  let stack = RcStack::new();
  stack.push(5).unwrap();
  assert_eq!(stack.peek(), Some(5));
  let _ = stack.pop();
  assert_eq!(stack.peek(), None);
}
