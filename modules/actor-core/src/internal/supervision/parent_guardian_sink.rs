use crate::{
  api::{actor::actor_ref::PriorityActorRef, failure::FailureInfo, mailbox::messages::SystemMessage},
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::{AnyMessage, MapSystemShared},
    supervision::EscalationSink,
  },
};

/// Sink that forwards `SystemMessage::Escalate` to parent Guardian.
pub(crate) struct ParentGuardianSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  control_ref: PriorityActorRef<AnyMessage, MF>,
  map_system:  MapSystemShared<AnyMessage>,
}

impl<MF> ParentGuardianSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  pub(crate) const fn new(
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) -> Self {
    Self { control_ref, map_system }
  }
}

impl<MF> EscalationSink<AnyMessage, MF> for ParentGuardianSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  fn handle(&mut self, info: FailureInfo, already_handled: bool) -> Result<(), FailureInfo> {
    if already_handled {
      return Ok(());
    }

    if let Some(parent_info) = info.escalate_to_parent() {
      let envelope =
        PriorityEnvelope::from_system(SystemMessage::Escalate(parent_info)).map(|sys| (*self.map_system)(sys));
      if self.control_ref.try_send_envelope_mailbox(envelope).is_ok() {
        return Ok(());
      }
    } else {
      let envelope =
        PriorityEnvelope::from_system(SystemMessage::Escalate(info.clone())).map(|sys| (*self.map_system)(sys));
      if self.control_ref.try_send_envelope_mailbox(envelope).is_ok() {
        return Ok(());
      }
    }

    Err(info)
  }
}
