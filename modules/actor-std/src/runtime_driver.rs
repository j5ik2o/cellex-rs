use core::convert::Infallible;
use core::marker::PhantomData;
use core::num::NonZeroUsize;

use cellex_actor_core_rs::{ActorRuntime, ActorSystemRunner, ShutdownToken};
use cellex_actor_core_rs::{ArcShared, PriorityEnvelope, ReadyQueueWorker, RuntimeMessage};
use cellex_utils_std_rs::QueueError;
use futures::future::select_all;
use tokio::signal;
use tokio::task::{self, JoinHandle};

/// Handle for managing the actor system in the Tokio execution environment
///
/// Controls the startup, shutdown, and termination waiting of the actor system.
pub struct TokioSystemHandle<U>
where
  U: cellex_utils_std_rs::Element, {
  join: tokio::task::JoinHandle<Result<Infallible, QueueError<PriorityEnvelope<RuntimeMessage>>>>,
  shutdown: ShutdownToken,
  _marker: PhantomData<U>,
}

impl<U> TokioSystemHandle<U>
where
  U: cellex_utils_std_rs::Element,
{
  /// Starts the actor system as a local task
  ///
  /// # Arguments
  /// * `runner` - The actor system runner to start
  ///
  /// # Returns
  /// A new `TokioSystemHandle` for managing the actor system
  pub fn start_local<R>(runner: ActorSystemRunner<U, R>) -> Self
  where
    U: cellex_utils_std_rs::Element + 'static,
    R: ActorRuntime + Clone + 'static, {
    let shutdown = runner.shutdown_token();
    let join = task::spawn_local(async move { run_runner(runner).await });
    Self {
      join,
      shutdown,
      _marker: PhantomData,
    }
  }

  /// Returns the system's shutdown token
  ///
  /// # Returns
  /// A `ShutdownToken` for controlling shutdown
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
  pub async fn await_terminated(
    self,
  ) -> Result<Result<Infallible, QueueError<PriorityEnvelope<RuntimeMessage>>>, tokio::task::JoinError> {
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
  pub fn spawn_ctrl_c_listener(&self) -> JoinHandle<()> {
    let token = self.shutdown.clone();
    tokio::spawn(async move {
      if signal::ctrl_c().await.is_ok() {
        token.trigger();
      }
    })
  }
}

async fn run_runner<U, R>(
  runner: ActorSystemRunner<U, R>,
) -> Result<Infallible, QueueError<PriorityEnvelope<RuntimeMessage>>>
where
  U: cellex_utils_std_rs::Element + 'static,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<RuntimeMessage>>: Clone,
  R::Signal: Clone, {
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

async fn run_ready_queue_workers<U, R>(
  runner: ActorSystemRunner<U, R>,
  worker_count: NonZeroUsize,
) -> Result<Infallible, QueueError<PriorityEnvelope<RuntimeMessage>>>
where
  U: cellex_utils_std_rs::Element + 'static,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<RuntimeMessage>>: Clone,
  R::Signal: Clone, {
  let shutdown = runner.shutdown_token();
  let mut worker_handles = Vec::with_capacity(worker_count.get());

  for _ in 0..worker_count.get() {
    let worker = runner
      .ready_queue_worker()
      .expect("ReadyQueue worker must be available when support is reported");
    worker_handles.push(spawn_worker_task(worker, shutdown.clone()));
  }

  let mut worker_handles = worker_handles;

  loop {
    if worker_handles.is_empty() {
      return Err(QueueError::Disconnected);
    }

    let (result, _index, mut remaining) = select_all(worker_handles).await;

    match result {
      Ok(Ok(())) => {
        if shutdown.is_triggered() {
          if remaining.is_empty() {
            return Err(QueueError::Disconnected);
          }
          worker_handles = remaining;
          continue;
        }

        let worker = runner
          .ready_queue_worker()
          .expect("ReadyQueue worker must be obtainable while scheduler remains active");
        remaining.push(spawn_worker_task(worker, shutdown.clone()));
        worker_handles = remaining;
      }
      Ok(Err(err)) => return Err(err),
      Err(join_err) => {
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
      }
    }
  }
}

fn spawn_worker_task<R>(
  worker: ArcShared<dyn ReadyQueueWorker<RuntimeMessage, R>>,
  shutdown: ShutdownToken,
) -> JoinHandle<Result<(), QueueError<PriorityEnvelope<RuntimeMessage>>>>
where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<RuntimeMessage>>: Clone,
  R::Signal: Clone, {
  task::spawn_local(async move { ready_queue_worker_loop(worker, shutdown).await })
}

async fn ready_queue_worker_loop<R>(
  worker: ArcShared<dyn ReadyQueueWorker<RuntimeMessage, R>>,
  shutdown: ShutdownToken,
) -> Result<(), QueueError<PriorityEnvelope<RuntimeMessage>>>
where
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<RuntimeMessage>>: Clone,
  R::Signal: Clone, {
  loop {
    if shutdown.is_triggered() {
      return Ok(());
    }

    if let Some(_) = worker.process_ready_once()? {
      task::yield_now().await;
      continue;
    }

    match worker.wait_for_ready() {
      Some(wait_future) => {
        tokio::select! {
          _ = wait_future => {}
          _ = wait_for_shutdown(shutdown.clone()) => {
            return Ok(());
          }
        }
      }
      None => {
        task::yield_now().await;
      }
    }
  }
}

async fn wait_for_shutdown(token: ShutdownToken) {
  while !token.is_triggered() {
    task::yield_now().await;
  }
}
