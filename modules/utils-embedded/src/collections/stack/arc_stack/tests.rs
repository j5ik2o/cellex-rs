#![allow(clippy::disallowed_types)]
use super::*;
use crate::tests::init_arc_critical_section;

fn prepare() {
  init_arc_critical_section();
}

#[test]
fn arc_stack_push_pop() {
  prepare();
  let stack: ArcStack<u32> = ArcLocalStack::with_capacity(1);
  stack.push(1).unwrap();
  assert!(stack.push(2).is_err());
  assert_eq!(stack.pop(), Some(1));
  assert!(stack.pop().is_none());
}

#[test]
fn arc_stack_handle_operations() {
  prepare();
  let stack: ArcStack<u32> = ArcLocalStack::new();
  stack.push(10).unwrap();
  let cloned = stack.clone();
  cloned.push(11).unwrap();

  assert_eq!(stack.len().to_usize(), 2);
  assert_eq!(cloned.pop(), Some(11));
  assert_eq!(stack.pop(), Some(10));
}

#[test]
fn arc_stack_peek_ref() {
  prepare();
  let stack: ArcStack<u32> = ArcLocalStack::new();
  stack.push(5).unwrap();
  assert_eq!(stack.peek(), Some(5));
  stack.pop();
  assert_eq!(stack.peek(), None);
}
