use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::TypeId;
use core::cell::RefCell;
use core::marker::PhantomData;

use crate::api::actor::PriorityActorRef;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::context::{ActorContext, ActorHandlerFn, ChildSpawnSpec};
use crate::internal::guardian::{Guardian, GuardianStrategy};
use crate::internal::mailbox::PriorityMailboxSpawnerHandle;
use crate::internal::metrics::MetricsSinkShared;
use crate::internal::scheduler::ReadyQueueHandle;
use crate::ActorId;
use crate::ActorPath;
use crate::DynMessage;
use crate::Extensions;
use crate::FailureInfo;
use crate::MailboxRuntime;
use crate::SpawnError;
use crate::Supervisor;
use crate::SystemMessage;
use crate::{ActorFailure, Mailbox, MailboxHandle, MailboxProducer};
use cellex_utils_core_rs::{Element, QueueError};

use crate::ReceiveTimeoutScheduler;
use crate::{MapSystemShared, ReceiveTimeoutFactoryShared};

pub(crate) struct ActorCell<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  #[cfg_attr(not(feature = "std"), allow(dead_code))]
  actor_id: ActorId,
  map_system: MapSystemShared<M>,
  watchers: Vec<ActorId>,
  actor_path: ActorPath,
  mailbox_runtime: R,
  mailbox_spawner: PriorityMailboxSpawnerHandle<M, R>,
  mailbox: R::Mailbox<PriorityEnvelope<M>>,
  sender: R::Producer<PriorityEnvelope<M>>,
  supervisor: Box<dyn Supervisor<M>>,
  handler: Box<ActorHandlerFn<M, R>>,
  _strategy: PhantomData<Strat>,
  stopped: bool,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
  receive_timeout_scheduler: Option<RefCell<Box<dyn ReceiveTimeoutScheduler>>>,
  extensions: Extensions,
}

mod dispatch;
mod processing;
mod setup;
mod timeout;
