use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use cellex_utils_core_rs::sync::ArcShared;

use super::{DeadLetter, DeadLetterHub, DeadLetterListener, DeadLetterReason};
use crate::api::{
  actor::ActorPath,
  process::pid::{Pid, SystemId},
};

#[test]
fn publishes_to_listeners() {
  let pid = Pid::new(SystemId::new("sys"), ActorPath::new());
  let mut hub = DeadLetterHub::new();
  let flag = Arc::new(AtomicBool::new(false));
  let flag_clone = Arc::clone(&flag);
  let listener = ArcShared::new(move |_letter: &DeadLetter<&'static str>| {
    flag_clone.store(true, Ordering::SeqCst);
  })
  .into_dyn(|f| f as &DeadLetterListener<_>);

  hub.subscribe(listener);

  hub.publish(&DeadLetter::new(pid, "msg", DeadLetterReason::UnregisteredPid));
  assert!(flag.load(Ordering::SeqCst));
}
