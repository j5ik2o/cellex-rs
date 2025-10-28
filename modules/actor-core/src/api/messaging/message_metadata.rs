use cellex_utils_core_rs::collections::Element;

use crate::{
  api::{
    mailbox::ThreadSafe,
    messaging::{MessageSender, MetadataStorageMode},
    process::pid::Pid,
  },
  internal::message::InternalMessageMetadata,
};

/// Typed metadata for the external API.
#[derive(Debug, Clone)]
pub struct MessageMetadata<C: MetadataStorageMode = ThreadSafe> {
  inner: InternalMessageMetadata<C>,
}

impl<C> MessageMetadata<C>
where
  C: MetadataStorageMode,
{
  /// Creates new empty metadata.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the sender and returns self (builder pattern).
  ///
  /// # Arguments
  /// * `sender` - Sender's dispatcher to set
  #[must_use]
  pub fn with_sender<U>(mut self, sender: MessageSender<U, C>) -> Self
  where
    U: Element, {
    self.inner = self.inner.with_sender(Some(sender.into_internal()));
    self
  }

  /// Sets the responder and returns self (builder pattern).
  ///
  /// # Arguments
  /// * `responder` - Responder's dispatcher to set
  #[must_use]
  pub fn with_responder<U>(mut self, responder: MessageSender<U, C>) -> Self
  where
    U: Element, {
    self.inner = self.inner.with_responder(Some(responder.into_internal()));
    self
  }

  /// Sets the sender PID and returns self.
  #[must_use]
  pub fn with_sender_pid(mut self, sender_pid: Pid) -> Self {
    self.inner = self.inner.with_sender_pid(Some(sender_pid));
    self
  }

  /// Sets the responder PID and returns self.
  #[must_use]
  pub fn with_responder_pid(mut self, responder_pid: Pid) -> Self {
    self.inner = self.inner.with_responder_pid(Some(responder_pid));
    self
  }

  /// Gets the sender dispatcher of the specified type.
  ///
  /// # Returns
  /// `Some(MessageSender<U>)` if sender exists, `None` otherwise
  pub fn sender_as<U>(&self) -> Option<MessageSender<U, C>>
  where
    U: Element, {
    self.inner.sender_cloned().map(MessageSender::new)
  }

  /// Gets the responder dispatcher of the specified type.
  ///
  /// # Returns
  /// `Some(MessageSender<U>)` if responder exists, `None` otherwise
  pub fn responder_as<U>(&self) -> Option<MessageSender<U, C>>
  where
    U: Element, {
    self.inner.responder_cloned().map(MessageSender::new)
  }

  /// Returns the sender PID if set.
  #[must_use]
  pub const fn sender_pid(&self) -> Option<&Pid> {
    self.inner.sender_pid()
  }

  /// Returns the responder PID if set.
  #[must_use]
  pub const fn responder_pid(&self) -> Option<&Pid> {
    self.inner.responder_pid()
  }

  /// Gets the dispatcher of the specified type (prioritizing responder).
  ///
  /// Returns the responder if it exists, otherwise returns the sender.
  ///
  /// # Returns
  /// `Some(MessageSender<U>)` if dispatcher exists, `None` otherwise
  #[must_use]
  pub fn dispatcher_for<U>(&self) -> Option<MessageSender<U, C>>
  where
    U: Element, {
    self.responder_as::<U>().or_else(|| self.sender_as::<U>())
  }

  /// Determines if the metadata is empty.
  ///
  /// # Returns
  /// `true` if neither sender nor responder exists, `false` otherwise
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.inner.sender.is_none()
      && self.inner.responder.is_none()
      && self.inner.sender_pid().is_none()
      && self.inner.responder_pid().is_none()
  }
}

impl<C> Default for MessageMetadata<C>
where
  C: MetadataStorageMode,
{
  fn default() -> Self {
    Self { inner: InternalMessageMetadata::default() }
  }
}
