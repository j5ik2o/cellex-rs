mod failure_event_handler_shared;
mod failure_event_listener_shared;
mod failure_telemetry_builder_shared;
mod failure_telemetry_shared;
mod telemetry_builder_fn;
mod telemetry_context;

pub use failure_event_handler_shared::FailureEventHandlerShared;
pub use failure_event_listener_shared::FailureEventListenerShared;
pub use failure_telemetry_builder_shared::FailureTelemetryBuilderShared;
pub use failure_telemetry_shared::FailureTelemetryShared;
#[allow(unused_imports)]
pub(crate) use telemetry_builder_fn::TelemetryBuilderFn;
pub use telemetry_context::TelemetryContext;

#[cfg(test)]
mod tests {
  use crate::api::extensions::Extensions;
  use crate::api::failure_telemetry::{FailureTelemetryBuilderShared, FailureTelemetryShared, TelemetryContext};
  use crate::api::supervision::telemetry::NoopFailureTelemetry;

  #[test]
  fn telemetry_builder_shared_invokes_closure() {
    let extensions = Extensions::new();
    let builder = FailureTelemetryBuilderShared::new(|_ctx| FailureTelemetryShared::new(NoopFailureTelemetry));
    let ctx = TelemetryContext::new(None, extensions);

    let telemetry = builder.build(&ctx);
    telemetry.with_ref(|_impl| {});
  }
}
