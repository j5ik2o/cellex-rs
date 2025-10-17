/// Shared failure telemetry infrastructure
pub mod failure_telemetry;
/// System message mapping functionality
pub mod map_system;
/// Receive timeout handling
pub mod receive_timeout;

#[cfg(test)]
mod tests {
  use super::failure_telemetry::{FailureTelemetryBuilderShared, FailureTelemetryShared, TelemetryContext};
  use crate::api::extensions::Extensions;
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
