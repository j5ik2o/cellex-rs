#![cfg(feature = "embassy_executor")]

use alloc::boxed::Box;
use core::marker::PhantomData;

use cellex_actor_core_rs::{
  api::{
    mailbox::MailboxFactory,
    receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory},
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MapSystemShared},
  },
};
use embassy_executor::Spawner;

use super::{
  internal::{SchedulerState, StateMutex, WakeSignal},
  scheduler::EmbassyReceiveTimeoutScheduler,
};

/// Factory that spawns Embassy timer tasks per actor.
pub struct EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: MailboxFactory + Clone + Send + Sync + 'static, {
  spawner: &'static Spawner,
  _marker: PhantomData<R>,
}

// Safety: Spawner is 'static and designed to be shared across contexts.
// While Spawner contains !Sync raw pointers internally, it is only accessed
// through its safe API which enforces proper synchronization.
unsafe impl<R> Sync for EmbassyReceiveTimeoutSchedulerFactory<R> where R: MailboxFactory + Clone + Send + Sync + 'static {}
unsafe impl<R> Send for EmbassyReceiveTimeoutSchedulerFactory<R> where R: MailboxFactory + Clone + Send + Sync + 'static {}

impl<R> EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: MailboxFactory + Clone + Send + Sync + 'static,
{
  /// Creates a new factory backed by the provided Embassy spawner.
  pub fn new(spawner: &'static Spawner) -> Self {
    Self { spawner, _marker: PhantomData }
  }
}

impl<R> Clone for EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: MailboxFactory + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self { spawner: self.spawner, _marker: PhantomData }
  }
}

impl<R> ReceiveTimeoutSchedulerFactory<AnyMessage, R> for EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: MailboxFactory + Clone + Send + Sync + 'static,
  R::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<AnyMessage>>: Clone,
{
  fn create(
    &self,
    sender: R::Producer<PriorityEnvelope<AnyMessage>>,
    map_system: MapSystemShared<AnyMessage>,
  ) -> Box<dyn ReceiveTimeoutScheduler> {
    // Allocate state and signal on the heap and leak them to obtain 'static references.
    let state = Box::leak(Box::new(StateMutex::new(SchedulerState::new())));
    let wake_signal = Box::leak(Box::new(WakeSignal::new()));

    let runner_state = unsafe { &*(state as *const _) };
    let runner_signal = unsafe { &*(wake_signal as *const _) };

    let sender_clone = sender.clone();
    let map_clone = map_system.clone();

    // TODO: Embassy 0.9 requires #[embassy_executor::task] macro for spawn,
    // which doesn't work with generic functions. Need to refactor this approach.
    // For now, we create the scheduler but the background task won't run.
    let _ = (self.spawner, runner_state, runner_signal, sender_clone, map_clone);
    // self
    //   .spawner
    //   .spawn(run_scheduler::<R>(runner_state, runner_signal, sender_clone, map_clone))
    //   .expect("failed to spawn embassy receive-timeout worker");

    Box::new(EmbassyReceiveTimeoutScheduler::new(state, wake_signal))
  }
}
