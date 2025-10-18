use alloc::boxed::Box;
use core::marker::PhantomData;

use cellex_utils_core_rs::{Element, QueueError};

use crate::api::{
  mailbox::{MailboxFactory, PriorityEnvelope},
  supervision::failure::FailureInfo,
};

type FailureHandler<M> = dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static;

use crate::api::supervision::escalation::EscalationSink;

/// Sink based on custom handler.
pub(crate) struct CustomEscalationSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  handler: Box<FailureHandler<M>>,
  _marker: PhantomData<MF>,
}

impl<M, MF> CustomEscalationSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
{
  pub(crate) fn new<F>(handler: F) -> Self
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    Self { handler: Box::new(handler), _marker: PhantomData }
  }
}

impl<M, MF> EscalationSink<M, MF> for CustomEscalationSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
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
