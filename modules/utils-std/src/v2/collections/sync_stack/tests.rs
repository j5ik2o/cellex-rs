use cellex_utils_core_rs::v2::collections::stack::{PushOutcome, StackError, StackOverflowPolicy};

use super::*;

#[test]
fn push_pop_roundtrip() {
  let stack = make_std_vec_stack_blocking::<u32>(4);
  assert!(matches!(stack.push(1), Ok(PushOutcome::Pushed)));
  assert!(matches!(stack.push(2), Ok(PushOutcome::Pushed)));
  assert_eq!(stack.len(), 2);
  assert_eq!(stack.pop().unwrap(), 2);
  assert_eq!(stack.pop().unwrap(), 1);
  assert!(matches!(stack.pop(), Err(StackError::Empty)));
}

#[test]
fn grow_policy_expands_capacity() {
  let stack = make_std_vec_stack::<u32>(1, StackOverflowPolicy::Grow);
  assert!(matches!(stack.push(1), Ok(PushOutcome::Pushed)));
  assert!(matches!(stack.push(2), Ok(PushOutcome::GrewTo { capacity: 2 })));
  assert_eq!(stack.capacity(), 2);
}
