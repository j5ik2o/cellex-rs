//! `TokioReceiveTimeoutScheduler` implementation.

use core::time::Duration;

use cellex_actor_core_rs::{
  api::{actor_system::map_system::MapSystemShared, receive_timeout::ReceiveTimeoutScheduler},
  shared::messaging::AnyMessage,
};
use tokio::{
  sync::mpsc::{unbounded_channel, UnboundedSender},
  task::JoinHandle,
};

use super::shared::{run_scheduler, Command, TokioSender};

/// Scheduler that drives `ReceiveTimeout` on Tokio runtime.
///
/// Spawns a dedicated task that polls `TokioDeadlineTimer` and sends
/// `PriorityEnvelope<SystemMessage>` to the priority mailbox when expired.
/// The `ActorCell` side can simply call `set` / `cancel` / `notify_activity`
/// without being aware of the timer implementation.
pub struct TokioReceiveTimeoutScheduler {
  pub(super) tx:     UnboundedSender<Command>,
  pub(super) handle: JoinHandle<()>,
}

impl TokioReceiveTimeoutScheduler {
  /// Creates a new scheduler and spawns its background task.
  pub(super) fn new(sender: TokioSender, map_system: MapSystemShared<AnyMessage>) -> Self {
    let (tx, handle) = Self::spawn_task(sender, map_system);
    Self { tx, handle }
  }

  pub(super) fn spawn_task(
    sender: TokioSender,
    map_system: MapSystemShared<AnyMessage>,
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
