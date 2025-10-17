use crate::api::mailbox::MailboxRuntime;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::supervision::escalation::EscalationSink;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::api::supervision::escalation::RootEscalationSink;
use crate::api::supervision::failure::FailureInfo;
use crate::api::supervision::telemetry::TelemetryObservationConfig;
use crate::internal::actor::InternalActorRef;
use crate::shared::failure_telemetry::FailureTelemetryShared;
use crate::shared::map_system::MapSystemShared;
use cellex_utils_core_rs::{Element, QueueError};

use super::{CustomEscalationSink, ParentGuardianSink};

/// Composes multiple sinks and applies them in order.
pub(crate) struct CompositeEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  parent_guardian: Option<ParentGuardianSink<M, R>>,
  custom: Option<CustomEscalationSink<M, R>>,
  root: Option<RootEscalationSink<M, R>>,
}

impl<M, R> CompositeEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  pub(crate) fn new() -> Self {
    Self {
      parent_guardian: None,
      custom: None,
      root: Some(RootEscalationSink::<M, R>::new()),
    }
  }

  pub(crate) fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    self.parent_guardian = Some(ParentGuardianSink::new(control_ref, map_system));
  }

  pub(crate) fn set_custom_handler<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    self.custom = Some(CustomEscalationSink::new(handler));
  }

  pub(crate) fn set_root_handler(&mut self, handler: Option<FailureEventHandler>) {
    if let Some(root) = self.root.as_mut() {
      root.set_event_handler(handler);
    } else {
      let mut sink = RootEscalationSink::<M, R>::new();
      sink.set_event_handler(handler);
      self.root = Some(sink);
    }
  }

  pub(crate) fn set_root_listener(&mut self, listener: Option<FailureEventListener>) {
    if let Some(root) = self.root.as_mut() {
      root.set_event_listener(listener);
    } else if let Some(listener) = listener {
      let mut sink = RootEscalationSink::<M, R>::new();
      sink.set_event_listener(Some(listener));
      self.root = Some(sink);
    }
  }

  pub(crate) fn set_root_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    if let Some(root) = self.root.as_mut() {
      root.set_telemetry(telemetry);
    } else {
      let mut sink = RootEscalationSink::<M, R>::new();
      sink.set_telemetry(telemetry);
      self.root = Some(sink);
    }
  }

  pub(crate) fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    if let Some(root) = self.root.as_mut() {
      root.set_observation_config(config);
    } else {
      let mut sink = RootEscalationSink::<M, R>::new();
      sink.set_observation_config(config);
      self.root = Some(sink);
    }
  }
}

impl<M, R> Default for CompositeEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<M, R> EscalationSink<M, R> for CompositeEscalationSink<M, R>
where
  M: Element,
  R: MailboxRuntime,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  fn handle(&mut self, info: FailureInfo, already_handled: bool) -> Result<(), FailureInfo> {
    let mut handled = already_handled;
    let mut last_failure = info;

    if let Some(parent) = self.parent_guardian.as_mut() {
      match parent.handle(last_failure.clone(), handled) {
        Ok(()) => handled = true,
        Err(unhandled) => {
          last_failure = unhandled;
          handled = false;
        }
      }
    }

    if let Some(custom) = self.custom.as_mut() {
      match custom.handle(last_failure.clone(), handled) {
        Ok(()) => handled = true,
        Err(unhandled) => {
          last_failure = unhandled;
          handled = false;
        }
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
