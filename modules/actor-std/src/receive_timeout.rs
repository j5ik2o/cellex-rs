//! ReceiveTimeout scheduler implementation for Tokio runtime.
//!
//! Combines `TokioDeadlineTimer` with priority mailboxes to provide
//! a mechanism for delivering `SystemMessage::ReceiveTimeout` to actors.

use core::time::Duration;

use cellex_actor_core_rs::api::{
  actor_system::map_system::MapSystemShared,
  mailbox::{MailboxFactory, PriorityEnvelope, SystemMessage},
  messaging::DynMessage,
  receive_timeout::{
    ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory, ReceiveTimeoutSchedulerFactoryProvider,
    ReceiveTimeoutSchedulerFactoryShared,
  },
};
use cellex_utils_std_rs::{DeadlineTimer, DeadlineTimerExpired, DeadlineTimerKey, TimerDeadline, TokioDeadlineTimer};
use futures::future::poll_fn;
use tokio::{
  sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
  task::JoinHandle,
};

use crate::TokioMailboxRuntime;

/// Producer for sending `PriorityEnvelope<DynMessage>` to Tokio mailbox.
type TokioSender = <TokioMailboxRuntime as MailboxFactory>::Producer<PriorityEnvelope<DynMessage>>;

#[derive(Debug)]
enum Command {
  Set(Duration),
  Cancel,
  Reset,
  Shutdown,
}

struct TimerState {
  key:      Option<DeadlineTimerKey>,
  duration: Option<Duration>,
}

impl TimerState {
  fn new() -> Self {
    Self { key: None, duration: None }
  }
}

/// Scheduler that drives `ReceiveTimeout` on Tokio runtime.
///
/// Spawns a dedicated task that polls `TokioDeadlineTimer` and sends
/// `PriorityEnvelope<SystemMessage>` to the priority mailbox when expired.
/// The `ActorCell` side can simply call `set` / `cancel` / `notify_activity`
/// without being aware of the timer implementation.
pub struct TokioReceiveTimeoutScheduler {
  tx:     UnboundedSender<Command>,
  handle: JoinHandle<()>,
}

impl TokioReceiveTimeoutScheduler {
  fn spawn_task(
    sender: TokioSender,
    map_system: MapSystemShared<DynMessage>,
  ) -> (UnboundedSender<Command>, JoinHandle<()>) {
    let (tx, rx) = unbounded_channel();
    let handle = tokio::spawn(run_scheduler(rx, sender, map_system));
    (tx, handle)
  }
}

impl ReceiveTimeoutScheduler for TokioReceiveTimeoutScheduler {
  fn set(&mut self, duration: Duration) {
    let _ = self.tx.send(Command::Set(duration));
  }

  fn cancel(&mut self) {
    let _ = self.tx.send(Command::Cancel);
  }

  fn notify_activity(&mut self) {
    let _ = self.tx.send(Command::Reset);
  }
}

impl Drop for TokioReceiveTimeoutScheduler {
  fn drop(&mut self) {
    let _ = self.tx.send(Command::Shutdown);
    self.handle.abort();
  }
}

/// `ReceiveTimeoutSchedulerFactory` implementation for Tokio runtime.
///
/// Receives the priority mailbox producer and SystemMessage conversion closure,
/// spawns an internal scheduler task, and returns a `ReceiveTimeoutScheduler`.
/// Assigning it via `ActorSystemConfig::with_receive_timeout_factory` or
/// `ActorSystemConfig::set_receive_timeout_scheduler_factory_shared_opt` enables
/// `ReceiveTimeout` support for the Tokio runtime.
pub struct TokioReceiveTimeoutSchedulerFactory;

impl TokioReceiveTimeoutSchedulerFactory {
  /// Creates a new factory.
  pub fn new() -> Self {
    Self
  }
}

impl Default for TokioReceiveTimeoutSchedulerFactory {
  fn default() -> Self {
    Self::new()
  }
}

impl ReceiveTimeoutSchedulerFactory<DynMessage, TokioMailboxRuntime> for TokioReceiveTimeoutSchedulerFactory {
  fn create(&self, sender: TokioSender, map_system: MapSystemShared<DynMessage>) -> Box<dyn ReceiveTimeoutScheduler> {
    let (tx, handle) = TokioReceiveTimeoutScheduler::spawn_task(sender, map_system);
    Box::new(TokioReceiveTimeoutScheduler { tx, handle })
  }
}

/// Runtime driver that provisions Tokio receive-timeout factories on demand.
#[derive(Debug, Default, Clone)]
pub struct TokioReceiveTimeoutDriver;

impl TokioReceiveTimeoutDriver {
  /// Creates a new driver instance.
  #[must_use]
  pub fn new() -> Self {
    Self
  }
}

impl ReceiveTimeoutSchedulerFactoryProvider<TokioMailboxRuntime> for TokioReceiveTimeoutDriver {
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<DynMessage, TokioMailboxRuntime> {
    ReceiveTimeoutSchedulerFactoryShared::new(TokioReceiveTimeoutSchedulerFactory::new())
  }
}

async fn wait_for_expired(timer: &mut TokioDeadlineTimer<()>) -> DeadlineTimerExpired<()> {
  poll_fn(|cx| timer.poll_expired(cx)).await.expect("poll expired")
}

async fn run_scheduler(
  mut commands: UnboundedReceiver<Command>,
  sender: TokioSender,
  map_system: MapSystemShared<DynMessage>,
) {
  let mut timer = TokioDeadlineTimer::new();
  let mut state = TimerState::new();

  loop {
    tokio::select! {
      cmd = commands.recv() => {
        match cmd {
          Some(Command::Set(duration)) => {
            state.duration = Some(duration);
            match state.key {
              Some(key) => {
                let _ = timer.reset(key, TimerDeadline::from(duration));
              }
              None => {
                if let Ok(key) = timer.insert((), TimerDeadline::from(duration)) {
                  state.key = Some(key);
                }
              }
            }
          }
          Some(Command::Cancel) => {
            if let Some(key) = state.key.take() {
              let _ = timer.cancel(key);
            }
            state.duration = None;
          }
          Some(Command::Reset) => {
            if let (Some(key), Some(duration)) = (state.key, state.duration) {
              let _ = timer.reset(key, TimerDeadline::from(duration));
            }
          }
          Some(Command::Shutdown) | None => {
            break;
          }
        }
      }
      expired = wait_for_expired(&mut timer), if state.key.is_some() => {
        let _ = expired;
        state.key = None;
        let envelope = PriorityEnvelope::from_system(SystemMessage::ReceiveTimeout)
          .map(|sys| (map_system)(sys));
        let _ = sender.try_send(envelope);
        if let Some(duration) = state.duration {
          if let Ok(key) = timer.insert((), TimerDeadline::from(duration)) {
            state.key = Some(key);
          }
        }
      }
    }
  }
}
