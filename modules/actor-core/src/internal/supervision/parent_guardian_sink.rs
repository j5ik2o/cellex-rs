use crate::api::actor::actor_ref::PriorityActorRef;
use crate::api::actor_system::map_system::MapSystemShared;
use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::MailboxProducer;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::mailbox::SystemMessage;
use crate::api::supervision::escalation::EscalationSink;
use crate::api::supervision::failure::FailureInfo;
use cellex_utils_core_rs::Element;

/// Sink that forwards `SystemMessage::Escalate` to parent Guardian.
pub(crate) struct ParentGuardianSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone, {
  control_ref: PriorityActorRef<M, MF>,
  map_system: MapSystemShared<M>,
}

impl<M, MF> ParentGuardianSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
{
  pub(crate) const fn new(control_ref: PriorityActorRef<M, MF>, map_system: MapSystemShared<M>) -> Self {
    Self {
      control_ref,
      map_system,
    }
  }
}

impl<M, MF> EscalationSink<M, MF> for ParentGuardianSink<M, MF>
where
  M: Element,
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
{
  fn handle(&mut self, info: FailureInfo, already_handled: bool) -> Result<(), FailureInfo> {
    if already_handled {
      return Ok(());
    }

    if let Some(parent_info) = info.escalate_to_parent() {
      let envelope =
        PriorityEnvelope::from_system(SystemMessage::Escalate(parent_info)).map(|sys| (&*self.map_system)(sys));
      if self.control_ref.sender().try_send(envelope).is_ok() {
        return Ok(());
      }
    } else {
      let envelope =
        PriorityEnvelope::from_system(SystemMessage::Escalate(info.clone())).map(|sys| (&*self.map_system)(sys));
      if self.control_ref.sender().try_send(envelope).is_ok() {
        return Ok(());
      }
    }

    Err(info)
  }
}
