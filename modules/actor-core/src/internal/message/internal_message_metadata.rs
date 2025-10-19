use crate::{
  api::{
    mailbox::{MailboxConcurrency, ThreadSafe},
    process::pid::Pid,
  },
  internal::message::internal_message_sender::InternalMessageSender,
};

/// Metadata accompanying a message (internal representation).
#[derive(Debug, Clone)]
pub(crate) struct InternalMessageMetadata<C: MailboxConcurrency = ThreadSafe> {
  pub(crate) sender:        Option<InternalMessageSender<C>>,
  pub(crate) responder:     Option<InternalMessageSender<C>>,
  pub(crate) sender_pid:    Option<Pid>,
  pub(crate) responder_pid: Option<Pid>,
}

impl<C> InternalMessageMetadata<C>
where
  C: MailboxConcurrency,
{
  /// Creates new metadata with sender and responder.
  ///
  /// # Arguments
  /// * `sender` - Sender's dispatcher (optional)
  /// * `responder` - Responder's dispatcher (optional)
  #[allow(dead_code)]
  pub const fn new(sender: Option<InternalMessageSender<C>>, responder: Option<InternalMessageSender<C>>) -> Self {
    Self { sender, responder, sender_pid: None, responder_pid: None }
  }

  /// Gets a reference to the sender's dispatcher.
  ///
  /// # Returns
  /// `Some(&InternalMessageSender)` if sender exists, `None` otherwise
  #[allow(dead_code)]
  pub const fn sender(&self) -> Option<&InternalMessageSender<C>> {
    self.sender.as_ref()
  }

  /// Gets a clone of the sender's dispatcher.
  ///
  /// # Returns
  /// `Some(InternalMessageSender)` if sender exists, `None` otherwise
  pub fn sender_cloned(&self) -> Option<InternalMessageSender<C>> {
    self.sender.clone()
  }

  /// Gets a reference to the responder's dispatcher.
  ///
  /// # Returns
  /// `Some(&InternalMessageSender)` if responder exists, `None` otherwise
  #[allow(dead_code)]
  pub const fn responder(&self) -> Option<&InternalMessageSender<C>> {
    self.responder.as_ref()
  }

  /// Gets a clone of the responder's dispatcher.
  ///
  /// # Returns
  /// `Some(InternalMessageSender)` if responder exists, `None` otherwise
  pub fn responder_cloned(&self) -> Option<InternalMessageSender<C>> {
    self.responder.clone()
  }

  /// Sets the sender and returns self (builder pattern).
  ///
  /// # Arguments
  /// * `sender` - Sender's dispatcher to set
  pub fn with_sender(mut self, sender: Option<InternalMessageSender<C>>) -> Self {
    self.sender = sender;
    self
  }

  /// Sets the responder and returns self (builder pattern).
  ///
  /// # Arguments
  /// * `responder` - Responder's dispatcher to set
  pub fn with_responder(mut self, responder: Option<InternalMessageSender<C>>) -> Self {
    self.responder = responder;
    self
  }

  /// Sets the sender PID and returns self.
  pub fn with_sender_pid(mut self, sender_pid: Option<Pid>) -> Self {
    self.sender_pid = sender_pid;
    self
  }

  /// Sets the responder PID and returns self.
  pub fn with_responder_pid(mut self, responder_pid: Option<Pid>) -> Self {
    self.responder_pid = responder_pid;
    self
  }

  /// Returns a reference to the sender PID if present.
  pub const fn sender_pid(&self) -> Option<&Pid> {
    self.sender_pid.as_ref()
  }

  /// Returns a reference to the responder PID if present.
  pub const fn responder_pid(&self) -> Option<&Pid> {
    self.responder_pid.as_ref()
  }
}

impl<C> Default for InternalMessageMetadata<C>
where
  C: MailboxConcurrency,
{
  fn default() -> Self {
    Self { sender: None, responder: None, sender_pid: None, responder_pid: None }
  }
}
