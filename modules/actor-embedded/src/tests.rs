#![allow(clippy::disallowed_types)]

extern crate std;

use alloc::{format, rc::Rc, string::String, vec::Vec};
use core::{
  cell::RefCell,
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};
use std::sync::{Arc, Condvar, Mutex};

use cellex_actor_core_rs::api::{
  actor::Props,
  actor_runtime::GenericActorRuntime,
  actor_system::{ActorSystem, ActorSystemConfig},
};
use futures::task::{waker, ArcWake};

use super::LocalMailboxRuntime;

type TestResult<T = ()> = Result<T, String>;

fn block_on<F: Future>(mut future: F) -> F::Output {
  struct WaitCell {
    state: Mutex<bool>,
    cvar:  Condvar,
  }

  impl ArcWake for WaitCell {
    fn wake_by_ref(arc_self: &Arc<Self>) {
      let mut ready = arc_self.state.lock().unwrap_or_else(|err| err.into_inner());
      *ready = true;
      arc_self.cvar.notify_one();
    }
  }

  let cell = Arc::new(WaitCell { state: Mutex::new(false), cvar: Condvar::new() });
  let waker = waker(cell.clone());
  let mut cx = Context::from_waker(&waker);
  // Safety: we never move `future` after pinning.
  let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
  loop {
    match pinned.as_mut().poll(&mut cx) {
      | Poll::Ready(output) => break output,
      | Poll::Pending => {
        let mut ready = cell.state.lock().unwrap_or_else(|err| err.into_inner());
        while !*ready {
          ready = cell.cvar.wait(ready).unwrap_or_else(|err| err.into_inner());
        }
        *ready = false;
      },
    }
  }
}

#[test]
fn typed_actor_system_dispatch_next_processes_message() -> TestResult {
  let mailbox_factory = LocalMailboxRuntime::default();
  let actor_runtime = GenericActorRuntime::new(mailbox_factory);
  let mut system: ActorSystem<u32, _> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let props = Props::new(move |_, msg: u32| {
    log_clone.borrow_mut().push(msg);
    Ok(())
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).map_err(|err| format!("spawn typed actor: {:?}", err))?;

  actor_ref.tell(21).map_err(|err| format!("tell message: {:?}", err))?;

  block_on(async { root.dispatch_next().await }).map_err(|err| format!("dispatch next: {:?}", err))?;

  assert_eq!(log.borrow().as_slice(), &[21]);
  Ok(())
}
