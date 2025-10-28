use std::sync::Arc;

use cellex_actor_core_rs::shared::mailbox::MailboxSignal;
use tokio::sync::{futures::Notified, Notify};

/// A signal implementation using Tokio's `Notify` primitive
///
/// Provides an async notification mechanism for mailbox wake-ups.
#[derive(Clone, Debug)]
pub struct NotifySignal {
  inner: Arc<Notify>,
}

impl Default for NotifySignal {
  fn default() -> Self {
    Self { inner: Arc::new(Notify::new()) }
  }
}

impl MailboxSignal for NotifySignal {
  type WaitFuture<'a>
    = Notified<'a>
  where
    Self: 'a;

  fn notify(&self) {
    self.inner.notify_one();
  }

  fn wait(&self) -> Self::WaitFuture<'_> {
    self.inner.notified()
  }
}
