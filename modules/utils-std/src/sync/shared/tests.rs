#![allow(clippy::disallowed_types)]
use std::sync::Mutex;

use cellex_utils_core_rs::{QueueBase, RingBuffer};

use super::*;

#[test]
fn arc_shared_try_unwrap_behavior() {
  let shared = ArcShared::new(1_u32);
  assert_eq!(ArcShared::new(2_u32).try_unwrap().unwrap(), 2);
  let clone = shared.clone();
  assert!(clone.try_unwrap().is_err());
}

#[test]
fn arc_shared_queue_handle_storage_access() {
  let ring = RingBuffer::<u32>::new(1).with_dynamic(false);
  let storage = ArcShared::new(Mutex::new(ring));
  let handle = storage.storage();
  assert_eq!(handle.lock().unwrap().capacity().to_usize(), 1);
}

#[test]
fn arc_shared_conversions_round_trip() {
  let arc = Arc::new(7_u32);
  let shared = ArcShared::from_arc(arc.clone());
  assert!(Arc::ptr_eq(&shared.clone().into_arc(), &arc));
}
