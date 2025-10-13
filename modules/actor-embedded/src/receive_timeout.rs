#![cfg(feature = "embassy_executor")]

use core::marker::PhantomData;
use core::time::Duration;

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration as EmbassyDuration, Timer};

use cellex_actor_core_rs::{
  ActorRuntime, DynMessage, MapSystemShared, PriorityEnvelope, ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory,
  SystemMessage,
};

/// Internal shared state manipulated by both scheduler commands and the worker task.
struct SchedulerState {
  duration: Option<Duration>,
  generation: u32,
}

impl SchedulerState {
  const fn new() -> Self {
    Self {
      duration: None,
      generation: 0,
    }
  }

  fn increment_generation(&mut self) {
    self.generation = self.generation.wrapping_add(1);
  }
}

type StateMutex = Mutex<CriticalSectionRawMutex, SchedulerState>;
type WakeSignal = Signal<CriticalSectionRawMutex, ()>;

/// Receive-timeout scheduler used by the Embassy runtime.
///
/// Commands (`set` / `cancel` / `notify_activity`) update shared state immediately and
/// wake the background task, which drives the actual Embassy timer.
pub struct EmbassyReceiveTimeoutScheduler {
  state: &'static StateMutex,
  wake_signal: &'static WakeSignal,
}

impl EmbassyReceiveTimeoutScheduler {
  fn new(state: &'static StateMutex, wake_signal: &'static WakeSignal) -> Self {
    Self { state, wake_signal }
  }
}

impl ReceiveTimeoutScheduler for EmbassyReceiveTimeoutScheduler {
  fn set(&mut self, duration: Duration) {
    {
      let mut state = self.state.lock();
      state.duration = Some(duration);
      state.increment_generation();
    }
    self.wake_signal.signal(());
  }

  fn cancel(&mut self) {
    {
      let mut state = self.state.lock();
      state.duration = None;
      state.increment_generation();
    }
    self.wake_signal.signal(());
  }

  fn notify_activity(&mut self) {
    let mut should_signal = false;
    {
      let mut state = self.state.lock();
      if state.duration.is_some() {
        state.increment_generation();
        should_signal = true;
      }
    }
    if should_signal {
      self.wake_signal.signal(());
    }
  }
}

/// Factory that spawns Embassy timer tasks per actor.
pub struct EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: ActorRuntime + Clone + 'static, {
  spawner: &'static Spawner,
  _marker: PhantomData<R>,
}

impl<R> EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: ActorRuntime + Clone + 'static,
{
  /// Creates a new factory backed by the provided Embassy spawner.
  pub fn new(spawner: &'static Spawner) -> Self {
    Self {
      spawner,
      _marker: PhantomData,
    }
  }
}

impl<R> Clone for EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: ActorRuntime + Clone + 'static,
{
  fn clone(&self) -> Self {
    Self {
      spawner: self.spawner,
      _marker: PhantomData,
    }
  }
}

impl<R> ReceiveTimeoutSchedulerFactory<DynMessage, R> for EmbassyReceiveTimeoutSchedulerFactory<R>
where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  fn create(
    &self,
    sender: R::Producer<PriorityEnvelope<DynMessage>>,
    map_system: MapSystemShared<DynMessage>,
  ) -> Box<dyn ReceiveTimeoutScheduler> {
    // Allocate state and signal on the heap and leak them to obtain 'static references.
    let state = Box::leak(Box::new(StateMutex::new(SchedulerState::new())));
    let wake_signal = Box::leak(Box::new(WakeSignal::new()));

    let runner_state = unsafe { &*(state as *const _) };
    let runner_signal = unsafe { &*(wake_signal as *const _) };

    let sender_clone = sender.clone();
    let map_clone = map_system.clone();

    self
      .spawner
      .spawn(run_scheduler(runner_state, runner_signal, sender_clone, map_clone))
      .expect("failed to spawn embassy receive-timeout worker");

    Box::new(EmbassyReceiveTimeoutScheduler::new(state, wake_signal))
  }
}

fn to_embassy_duration(duration: Duration) -> EmbassyDuration {
  if duration.as_millis() == 0 {
    // Minimum resolution: 1 millisecond.
    EmbassyDuration::from_millis(1)
  } else {
    EmbassyDuration::from_millis(duration.as_millis() as u64)
  }
}

async fn run_scheduler<R>(
  state: &'static StateMutex,
  wake_signal: &'static WakeSignal,
  sender: R::Producer<PriorityEnvelope<DynMessage>>,
  map_system: MapSystemShared<DynMessage>,
) where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone, {
  loop {
    let (current_duration, generation) = {
      let state = state.lock();
      (state.duration, state.generation)
    };

    match current_duration {
      Some(duration) => {
        let mut timer = Timer::after(to_embassy_duration(duration));
        match select(timer, wake_signal.wait()).await {
          Either::First(_) => {
            let should_fire = {
              let mut state = state.lock();
              if state.duration.is_some() && state.generation == generation {
                state.increment_generation();
                true
              } else {
                false
              }
            };
            if should_fire {
              let envelope = PriorityEnvelope::from_system(SystemMessage::ReceiveTimeout).map(|sys| (map_system)(sys));
              let _ = sender.try_send(envelope);
              wake_signal.signal(());
            }
          }
          Either::Second(_) => {
            // Command received, restart loop to observe new state.
          }
        }
      }
      None => {
        wake_signal.wait().await;
      }
    }
  }
}
