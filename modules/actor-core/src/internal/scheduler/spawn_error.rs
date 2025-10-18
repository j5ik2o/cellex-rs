use alloc::string::String;

use cellex_utils_core_rs::{Element, QueueError};

use crate::api::mailbox::PriorityEnvelope;

/// Errors that can occur while spawning an actor through the scheduler.
#[derive(Debug)]
pub enum SpawnError<M>
where
  M: Element, {
  /// Underlying mailbox or queue failure.
  Queue(QueueError<PriorityEnvelope<M>>),
  /// Attempted to reuse an existing actor name.
  NameExists(String),
}

impl<M> SpawnError<M>
where
  M: Element,
{
  pub(crate) fn name_exists(name: impl Into<String>) -> Self {
    Self::NameExists(name.into())
  }
}

impl<M> From<QueueError<PriorityEnvelope<M>>> for SpawnError<M>
where
  M: Element,
{
  fn from(value: QueueError<PriorityEnvelope<M>>) -> Self {
    Self::Queue(value)
  }
}
