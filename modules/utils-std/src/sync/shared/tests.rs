#![allow(clippy::disallowed_types)]

use cellex_utils_core_rs::QueueBase;

use super::*;
use crate::collections::queue::MutexRingBufferStorage;

#[test]
fn arc_shared_try_unwrap_behavior() {
  let shared = ArcShared::new(1_u32);
  assert_eq!(ArcShared::new(2_u32).try_unwrap().unwrap(), 2);
  let clone = shared.clone();
  assert!(clone.try_unwrap().is_err());
}

#[test]
fn arc_shared_queue_handle_storage_access() {
  let storage = ArcShared::new(MutexRingBufferStorage::<u32>::with_capacity(1));
  let handle = storage.storage();
  handle.with_write(|buffer| buffer.set_dynamic(false));
  assert_eq!(handle.with_read(|buffer| buffer.capacity().to_usize()), 1);
}

#[test]
fn arc_shared_conversions_round_trip() {
  let arc = Arc::new(7_u32);
  let shared = ArcShared::from_arc(arc.clone());
  assert!(Arc::ptr_eq(&shared.clone().into_arc(), &arc));
}
