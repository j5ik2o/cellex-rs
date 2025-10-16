#![allow(deprecated, unused_imports)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::disallowed_types)]

use super::*;
use crate::api::mailbox::SystemMessage;
use crate::internal::guardian::AlwaysRestart;
use crate::internal::mailbox::test_support::TestMailboxRuntime;
use crate::DynMessage;
use crate::GenericActorRuntime;
use crate::MailboxOptions;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

use crate::MapSystemShared;
use cellex_utils_core_rs::{Element, DEFAULT_PRIORITY};
#[cfg(feature = "std")]
use futures::executor::block_on;

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
enum Message {
  User(u32),
  System,
}

#[cfg(feature = "std")]
#[test]
fn actor_system_spawns_and_processes_messages() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: InternalActorSystem<DynMessage, _, AlwaysRestart> = InternalActorSystem::new(actor_runtime);

  let map_system = MapSystemShared::new(|_: SystemMessage| DynMessage::new(Message::System));
  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let mut root = system.root_context();
  let actor_ref = root
    .spawn(InternalProps::new(
      MailboxOptions::default(),
      map_system.clone(),
      move |_, msg: DynMessage| {
        let Ok(message) = msg.downcast::<Message>() else {
          panic!("unexpected message type");
        };
        match message {
          Message::User(value) => log_clone.borrow_mut().push(value),
          Message::System => {}
        }
        Ok(())
      },
    ))
    .expect("spawn actor");

  actor_ref
    .try_send_with_priority(DynMessage::new(Message::User(7)), DEFAULT_PRIORITY)
    .expect("send message");

  block_on(root.dispatch_next()).expect("dispatch");

  assert_eq!(log.borrow().as_slice(), &[7]);
}
