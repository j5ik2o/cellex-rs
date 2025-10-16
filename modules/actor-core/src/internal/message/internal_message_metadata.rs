use crate::{InternalMessageSender, MailboxConcurrency, ThreadSafe};

/// Metadata accompanying a message (internal representation).
#[derive(Debug, Clone)]
pub struct InternalMessageMetadata<C: MailboxConcurrency = ThreadSafe> {
  pub(crate) sender: Option<InternalMessageSender<C>>,
  pub(crate) responder: Option<InternalMessageSender<C>>,
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
  pub fn new(sender: Option<InternalMessageSender<C>>, responder: Option<InternalMessageSender<C>>) -> Self {
    Self { sender, responder }
  }

  /// Gets a reference to the sender's dispatcher.
  ///
  /// # Returns
  /// `Some(&InternalMessageSender)` if sender exists, `None` otherwise
  pub fn sender(&self) -> Option<&InternalMessageSender<C>> {
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
  pub fn responder(&self) -> Option<&InternalMessageSender<C>> {
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
}

impl<C> Default for InternalMessageMetadata<C>
where
  C: MailboxConcurrency,
{
  fn default() -> Self {
    Self {
      sender: None,
      responder: None,
    }
  }
}
