#![allow(missing_docs)]

use futures::future::{select, Either, LocalBoxFuture};

use crate::api::actor::shutdown_token::ShutdownToken;
use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Worker interface exposing ReadyQueue operations for driver-level scheduling.
pub trait ReadyQueueWorker<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Processes one ready actor (if any). Returns `Some(true)` if progress was made.
  fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<M>>>;

  /// Returns a future that resolves when any actor becomes ready.
  fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>>;
}

/// Drives a single ReadyQueue worker loop until shutdown is triggered.
///
/// # Arguments
/// * `worker` - ReadyQueue worker instance
/// * `shutdown` - Shutdown signal token
/// * `yield_now` - Closure to yield execution
/// * `wait_for_shutdown` - Closure to wait for shutdown signal
pub async fn drive_ready_queue_worker<M, R, Y, YF, S, SF>(
  worker: ArcShared<dyn ReadyQueueWorker<M, R>>,
  shutdown: ShutdownToken,
  mut yield_now: Y,
  mut wait_for_shutdown: S,
) -> Result<(), QueueError<PriorityEnvelope<M>>>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Y: FnMut() -> YF,
  YF: core::future::Future<Output = ()>,
  S: FnMut() -> SF,
  SF: core::future::Future<Output = ()>, {
  loop {
    if shutdown.is_triggered() {
      return Ok(());
    }

    if let Some(progress) = worker.process_ready_once()? {
      if progress {
        yield_now().await;
        continue;
      }
    }

    match worker.wait_for_ready() {
      Some(wait_future) => {
        let shutdown_future = wait_for_shutdown();
        futures::pin_mut!(wait_future);
        futures::pin_mut!(shutdown_future);
        match select(wait_future, shutdown_future).await {
          Either::Left((_, _)) => {}
          Either::Right((_, _)) => return Ok(()),
        }
      }
      None => {
        yield_now().await;
      }
    }
  }
}
