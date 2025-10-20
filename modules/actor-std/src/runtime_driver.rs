use core::{convert::Infallible, marker::PhantomData, num::NonZeroUsize};

use cellex_actor_core_rs::api::{
  actor::shutdown_token::ShutdownToken,
  actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
  actor_scheduler::ready_queue_scheduler::{drive_ready_queue_worker, ReadyQueueWorker},
  actor_system::ActorSystemRunner,
  mailbox::messages::PriorityEnvelope,
  messaging::AnyMessage,
};
use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};
use futures::future::select_all;
use tokio::{
  signal,
  task::{self, JoinHandle},
};

/// Handle for managing the actor system in the Tokio execution environment
///
/// Controls the startup, shutdown, and termination waiting of the actor system.
pub struct TokioSystemHandle<U>
where
  U: Element, {
  join:     tokio::task::JoinHandle<Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>>,
  shutdown: ShutdownToken,
  _marker:  PhantomData<U>,
}

impl<U> TokioSystemHandle<U>
where
  U: Element,
{
  /// Starts the actor system as a local task
  ///
  /// # Arguments
  /// * `runner` - The actor system runner to start
  ///
  /// # Returns
  /// A new `TokioSystemHandle` for managing the actor system
  #[must_use]
  pub fn start_local<AR>(runner: ActorSystemRunner<U, AR>) -> Self
  where
    U: cellex_utils_std_rs::Element + 'static,
    AR: ActorRuntime + 'static,
    MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
    MailboxSignalOf<AR>: Clone, {
    let shutdown = runner.shutdown_token();
    let join = task::spawn_local(async move { run_runner(runner).await });
    Self { join, shutdown, _marker: PhantomData }
  }

  /// Returns the system's shutdown token
  ///
  /// # Returns
  /// A `ShutdownToken` for controlling shutdown
  #[must_use]
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.shutdown.clone()
  }

  /// Triggers the shutdown of the actor system
  ///
  /// Initiates a graceful shutdown of the system.
  pub fn trigger_shutdown(&self) {
    self.shutdown.trigger();
  }

  /// Waits for the actor system to terminate
  ///
  /// Asynchronously waits until the system has completely stopped.
  ///
  /// # Returns
  /// The result of system execution. The outer `Result` indicates task join errors,
  /// the inner `Result` indicates system execution errors.
  ///
  /// # Errors
  /// Returns a [`tokio::task::JoinError`] when the underlying task join fails.
  pub async fn await_terminated(
    self,
  ) -> Result<Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>, tokio::task::JoinError> {
    self.join.await
  }

  /// Forcibly terminates the actor system execution
  ///
  /// Aborts the system immediately without performing a graceful shutdown.
  pub fn abort(self) {
    self.join.abort();
  }

  /// Spawns a task that monitors Ctrl+C signals and triggers shutdown upon receipt
  ///
  /// # Returns
  /// A `JoinHandle` for the listener task
  #[must_use]
  pub fn spawn_ctrl_c_listener(&self) -> JoinHandle<()> {
    let token = self.shutdown.clone();
    tokio::spawn(async move {
      if signal::ctrl_c().await.is_ok() {
        token.trigger();
      }
    })
  }
}

async fn run_runner<U, AR>(
  runner: ActorSystemRunner<U, AR>,
) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>
where
  U: Element + 'static,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  if !runner.supports_ready_queue() {
    return runner.run_forever().await;
  }

  let worker_count = runner.ready_queue_worker_count();
  if worker_count.get() <= 1 {
    return runner.run_forever().await;
  }

  if runner.ready_queue_worker().is_none() {
    return runner.run_forever().await;
  }

  run_ready_queue_workers(runner, worker_count).await
}

async fn run_ready_queue_workers<U, AR>(
  runner: ActorSystemRunner<U, AR>,
  worker_count: NonZeroUsize,
) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>
where
  U: Element + 'static,
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  let shutdown = runner.shutdown_token();
  let mut worker_handles = Vec::with_capacity(worker_count.get());

  for _ in 0..worker_count.get() {
    let Some(worker) = runner.ready_queue_worker() else {
      return Err(QueueError::Disconnected);
    };
    worker_handles.push(spawn_worker_task::<AR>(worker, shutdown.clone()));
  }

  let mut worker_handles = worker_handles;

  loop {
    if worker_handles.is_empty() {
      return Err(QueueError::Disconnected);
    }

    let (result, _index, mut remaining) = select_all(worker_handles).await;

    match result {
      | Ok(Ok(())) => {
        if shutdown.is_triggered() {
          if remaining.is_empty() {
            return Err(QueueError::Disconnected);
          }
          worker_handles = remaining;
          continue;
        }

        if let Some(worker) = runner.ready_queue_worker() {
          remaining.push(spawn_worker_task::<AR>(worker, shutdown.clone()));
        }
        worker_handles = remaining;
      },
      | Ok(Err(err)) => return Err(err),
      | Err(join_err) => {
        if join_err.is_cancelled() && shutdown.is_triggered() {
          if remaining.is_empty() {
            return Err(QueueError::Disconnected);
          }
          worker_handles = remaining;
          continue;
        }

        if join_err.is_panic() {
          std::panic::resume_unwind(join_err.into_panic());
        }
        return Err(QueueError::Disconnected);
      },
    }
  }
}

fn spawn_worker_task<AR>(
  worker: ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>,
  shutdown: ShutdownToken,
) -> JoinHandle<Result<(), QueueError<PriorityEnvelope<AnyMessage>>>>
where
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  task::spawn_local(async move { ready_queue_worker_loop::<AR>(worker, shutdown).await })
}

async fn ready_queue_worker_loop<AR>(
  worker: ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>,
  shutdown: ShutdownToken,
) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
where
  AR: ActorRuntime + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone, {
  let shutdown_for_wait = shutdown.clone();
  drive_ready_queue_worker(worker, shutdown, task::yield_now, move || {
    wait_for_shutdown_signal(shutdown_for_wait.clone())
  })
  .await
}

async fn wait_for_shutdown_signal(token: ShutdownToken) {
  while !token.is_triggered() {
    task::yield_now().await;
  }
}
