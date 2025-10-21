//! Shared types and functions for receive timeout implementation.

use core::time::Duration;

use cellex_actor_core_rs::{
  api::mailbox::MailboxFactory,
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};
use cellex_utils_std_rs::{
  timing::TokioDeadlineTimer, DeadlineTimer, DeadlineTimerError, DeadlineTimerExpired, DeadlineTimerKey, TimerDeadline,
};
use futures::future::poll_fn;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::TokioMailboxRuntime;

/// Producer for sending `PriorityEnvelope<AnyMessage>` to Tokio mailbox.
pub(super) type TokioSender = <TokioMailboxRuntime as MailboxFactory>::Producer<PriorityEnvelope<AnyMessage>>;

#[derive(Debug)]
pub(super) enum Command {
  Set(Duration),
  Cancel,
  Reset,
  Shutdown,
}

pub(super) struct TimerState {
  pub(super) key:      Option<DeadlineTimerKey>,
  pub(super) duration: Option<Duration>,
}

impl TimerState {
  #[must_use]
  pub(super) const fn new() -> Self {
    Self { key: None, duration: None }
  }
}

pub(super) async fn wait_for_expired(
  timer: &mut TokioDeadlineTimer<()>,
) -> Result<DeadlineTimerExpired<()>, DeadlineTimerError> {
  poll_fn(|cx| timer.poll_expired(cx)).await
}

pub(super) async fn run_scheduler(
  mut commands: UnboundedReceiver<Command>,
  sender: TokioSender,
  map_system: cellex_actor_core_rs::shared::messaging::MapSystemShared<AnyMessage>,
) {
  use cellex_actor_core_rs::api::mailbox::messages::SystemMessage;

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
        match expired {
          | Ok(_) => {
            state.key = None;
            #[allow(clippy::redundant_closure)]
            let envelope = PriorityEnvelope::from_system(SystemMessage::ReceiveTimeout)
              .map(|sys| (map_system)(sys));
            let _ = sender.try_send(envelope);
            if let Some(duration) = state.duration {
              if let Ok(key) = timer.insert((), TimerDeadline::from(duration)) {
                state.key = Some(key);
              }
            }
          },
          | Err(_) => {
            state.key = None;
            state.duration = None;
          },
        }
      }
    }
  }
}
