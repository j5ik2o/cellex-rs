use cellex_utils_core_rs::collections::queue::QueueError;

use super::{CustomEscalationSink, ParentGuardianSink};
use crate::{
  api::{
    actor::actor_ref::PriorityActorRef,
    failure::{
      failure_event_stream::FailureEventListener,
      failure_telemetry::{FailureTelemetryObservationConfig, FailureTelemetryShared},
      FailureInfo,
    },
    mailbox::MailboxFactory,
    supervision::escalation::RootEscalationSink,
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MapSystemShared},
    supervision::{EscalationSink, FailureEventHandler},
  },
};

/// Composes multiple sinks and applies them in order.
pub(crate) struct CompositeEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  parent_guardian: Option<ParentGuardianSink<MF>>,
  custom:          Option<CustomEscalationSink<MF>>,
  root:            Option<RootEscalationSink<MF>>,
}

impl<MF> CompositeEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  pub(crate) fn new() -> Self {
    Self { parent_guardian: None, custom: None, root: Some(RootEscalationSink::<MF>::new()) }
  }

  pub(crate) fn set_parent_guardian(
    &mut self,
    control_ref: PriorityActorRef<AnyMessage, MF>,
    map_system: MapSystemShared<AnyMessage>,
  ) {
    self.parent_guardian = Some(ParentGuardianSink::new(control_ref, map_system));
  }

  pub(crate) fn set_custom_handler<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + 'static, {
    self.custom = Some(CustomEscalationSink::new(handler));
  }

  pub(crate) fn set_root_handler(&mut self, handler: Option<FailureEventHandler>) {
    if let Some(root) = self.root.as_mut() {
      root.set_failure_event_handler_opt(handler);
    } else {
      let mut sink = RootEscalationSink::<MF>::new();
      sink.set_failure_event_handler_opt(handler);
      self.root = Some(sink);
    }
  }

  pub(crate) fn set_root_listener(&mut self, listener: Option<FailureEventListener>) {
    if let Some(root) = self.root.as_mut() {
      root.set_failure_event_listener_opt(listener);
    } else if let Some(listener) = listener {
      let mut sink = RootEscalationSink::<MF>::new();
      sink.set_failure_event_listener_opt(Some(listener));
      self.root = Some(sink);
    }
  }

  pub(crate) fn set_root_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    if let Some(root) = self.root.as_mut() {
      root.set_failure_telemetry_shared(telemetry);
    } else {
      let mut sink = RootEscalationSink::<MF>::new();
      sink.set_failure_telemetry_shared(telemetry);
      self.root = Some(sink);
    }
  }

  pub(crate) fn set_root_observation_config(&mut self, config: FailureTelemetryObservationConfig) {
    if let Some(root) = self.root.as_mut() {
      root.set_failure_telemetry_observation_config(config);
    } else {
      let mut sink = RootEscalationSink::<MF>::new();
      sink.set_failure_telemetry_observation_config(config);
      self.root = Some(sink);
    }
  }
}

impl<MF> Default for CompositeEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<MF> EscalationSink<AnyMessage, MF> for CompositeEscalationSink<MF>
where
  MF: MailboxFactory,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  fn handle(&mut self, info: FailureInfo, already_handled: bool) -> Result<(), FailureInfo> {
    let mut handled = already_handled;
    let mut last_failure = info;

    if let Some(parent) = self.parent_guardian.as_mut() {
      match parent.handle(last_failure.clone(), handled) {
        | Ok(()) => handled = true,
        | Err(unhandled) => {
          last_failure = unhandled;
          handled = false;
        },
      }
    }

    if let Some(custom) = self.custom.as_mut() {
      match custom.handle(last_failure.clone(), handled) {
        | Ok(()) => handled = true,
        | Err(unhandled) => {
          last_failure = unhandled;
          handled = false;
        },
      }
    }

    if let Some(root) = self.root.as_mut() {
      let _ = root.handle(last_failure.clone(), handled);
    }

    if handled {
      Ok(())
    } else {
      Err(last_failure)
    }
  }
}
