#![allow(clippy::disallowed_types)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use cellex_utils_core_rs::sync::ArcShared;
use spin::Mutex;

use super::{DeadLetterReason, NodeId, Pid, ProcessRegistry, ProcessResolution, SystemId};
use crate::api::{
  actor::{ActorId, ActorPath},
  process::dead_letter::{DeadLetter, DeadLetterListener},
};

fn sample_path() -> ActorPath {
  ActorPath::new().push_child(ActorId(1)).push_child(ActorId(2))
}

#[test]
fn register_and_resolve_local() {
  let registry: ProcessRegistry<u32, usize> = ProcessRegistry::new(SystemId::new("sys"), None);
  let pid = registry.register_local(sample_path(), ArcShared::new(10));

  match registry.resolve_pid(&pid) {
    | ProcessResolution::Local(handle) => assert_eq!(*handle, 10),
    | other => panic!("unexpected resolution: {other:?}"),
  }
}

#[test]
fn detect_remote_pid() {
  let registry: ProcessRegistry<u32, usize> =
    ProcessRegistry::new(SystemId::new("sys"), Some(NodeId::new("node1", Some(2552))));
  let foreign_pid = Pid::new(SystemId::new("sys"), sample_path()).with_node(NodeId::new("node2", Some(2552)));
  assert!(matches!(registry.resolve_pid(&foreign_pid), ProcessResolution::Remote));
}

#[test]
fn publishes_dead_letter_when_unresolved() {
  let registry: ProcessRegistry<u32, i32> = ProcessRegistry::new(SystemId::new("sys"), None);
  let pid = Pid::new(SystemId::new("sys"), sample_path());

  let observed = Arc::new(AtomicBool::new(false));
  let observed_clone = Arc::clone(&observed);
  let listener = ArcShared::new(move |_: &DeadLetter<i32>| {
    observed_clone.store(true, Ordering::SeqCst);
  })
  .into_dyn(|f| f as &DeadLetterListener<i32>);
  registry.subscribe_dead_letters(listener);

  let result = registry.resolve_or_dead_letter(&pid, 5, DeadLetterReason::UnregisteredPid);
  assert!(result.is_none());
  assert!(observed.load(Ordering::SeqCst));
}

#[test]
fn publishes_dead_letter_when_remote() {
  let registry: ProcessRegistry<u32, i32> =
    ProcessRegistry::new(SystemId::new("sys"), Some(NodeId::new("node1", Some(2552))));
  let pid = Pid::new(SystemId::new("sys"), sample_path()).with_node(NodeId::new("node2", Some(2552)));

  let observed = Arc::new(Mutex::new(None));
  let observed_clone = Arc::clone(&observed);
  let listener = ArcShared::new(move |letter: &DeadLetter<i32>| {
    observed_clone.lock().replace(letter.reason.clone());
  })
  .into_dyn(|f| f as &DeadLetterListener<i32>);
  registry.subscribe_dead_letters(listener);

  let result = registry.resolve_or_dead_letter_with_remote(
    &pid,
    5,
    DeadLetterReason::UnregisteredPid,
    DeadLetterReason::NetworkUnreachable,
  );
  assert!(result.is_none());
  assert!(matches!(observed.lock().as_ref(), Some(DeadLetterReason::NetworkUnreachable)));
}
