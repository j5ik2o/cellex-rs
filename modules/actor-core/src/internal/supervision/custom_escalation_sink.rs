use alloc::boxed::Box;
use core::marker::PhantomData;

use cellex_utils_core_rs::QueueError;

use crate::api::{
  failure::FailureInfo,
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  messaging::AnyMessage,
};

type FailureHandler = dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static;

use crate::api::supervision::escalation::EscalationSink;

/// Sink based on custom handler.
pub(crate) struct CustomEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  handler: Box<FailureHandler>,
  _marker: PhantomData<MF>,
}

impl<MF> CustomEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  pub(crate) fn new<F>(handler: F) -> Self
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static, {
    Self { handler: Box::new(handler), _marker: PhantomData }
  }
}

impl<MF> EscalationSink<AnyMessage, MF> for CustomEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  fn handle(&mut self, info: FailureInfo, _already_handled: bool) -> Result<(), FailureInfo> {
    if (self.handler)(&info).is_ok() {
      Ok(())
    } else {
      Err(info)
    }
  }
}
