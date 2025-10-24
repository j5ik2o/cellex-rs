#![cfg(feature = "embassy_executor")]

use core::{cell::Cell, time::Duration};

use cellex_actor_core_rs::{
  api::mailbox::{messages::SystemMessage, MailboxFactory},
  shared::{
    mailbox::{messages::PriorityEnvelope, producer::MailboxProducer},
    messaging::{AnyMessage, MapSystemShared},
  },
};
use embassy_futures::select::{select, Either};
use embassy_sync::{
  blocking_mutex::{raw::CriticalSectionRawMutex, Mutex},
  signal::Signal,
};
use embassy_time::{Duration as EmbassyDuration, Timer};

/// Internal shared state manipulated by both scheduler commands and the worker task.
#[allow(dead_code)]
pub(super) struct SchedulerState {
  pub(super) duration:   Cell<Option<Duration>>,
  pub(super) generation: Cell<u32>,
}

impl SchedulerState {
  pub(super) const fn new() -> Self {
    Self { duration: Cell::new(None), generation: Cell::new(0) }
  }

  pub(super) fn increment_generation(&self) {
    self.generation.set(self.generation.get().wrapping_add(1));
  }

  pub(super) fn set_duration(&self, duration: Option<Duration>) {
    self.duration.set(duration);
  }

  pub(super) fn get_duration(&self) -> Option<Duration> {
    self.duration.get()
  }

  #[allow(dead_code)]
  pub(super) fn get_generation(&self) -> u32 {
    self.generation.get()
  }
}

pub(super) type StateMutex = Mutex<CriticalSectionRawMutex, SchedulerState>;
pub(super) type WakeSignal = Signal<CriticalSectionRawMutex, ()>;

#[allow(dead_code)]
pub(super) fn to_embassy_duration(duration: Duration) -> EmbassyDuration {
  if duration.as_millis() == 0 {
    // Minimum resolution: 1 millisecond.
    EmbassyDuration::from_millis(1)
  } else {
    EmbassyDuration::from_millis(duration.as_millis() as u64)
  }
}

#[allow(dead_code)]
pub(super) async fn run_scheduler<R>(
  state: &'static StateMutex,
  wake_signal: &'static WakeSignal,
  sender: R::Producer<PriorityEnvelope<AnyMessage>>,
  map_system: MapSystemShared<AnyMessage>,
) where
  R: MailboxFactory + Clone + Send + Sync + 'static,
  R::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<AnyMessage>>: Clone, {
  loop {
    let (current_duration, generation) = state.lock(|state| (state.get_duration(), state.get_generation()));

    match current_duration {
      | Some(duration) => {
        let timer = Timer::after(to_embassy_duration(duration));
        match select(timer, wake_signal.wait()).await {
          | Either::First(_) => {
            let should_fire = state.lock(|state| {
              if state.get_duration().is_some() && state.get_generation() == generation {
                state.increment_generation();
                true
              } else {
                false
              }
            });
            if should_fire {
              let envelope = PriorityEnvelope::from_system(SystemMessage::ReceiveTimeout).map(|sys| (map_system)(sys));
              let _ = sender.try_send(envelope);
              wake_signal.signal(());
            }
          },
          | Either::Second(_) => {
            // Command received, restart loop to observe new state.
          },
        }
      },
      | None => {
        wake_signal.wait().await;
      },
    }
  }
}
