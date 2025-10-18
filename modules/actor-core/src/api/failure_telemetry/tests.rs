use super::{FailureTelemetryBuilderShared, FailureTelemetryShared, TelemetryContext};
use crate::api::{extensions::Extensions, supervision::telemetry::NoopFailureTelemetry};

#[test]
fn telemetry_builder_shared_invokes_closure() {
  let extensions = Extensions::new();
  let builder = FailureTelemetryBuilderShared::new(|_ctx| FailureTelemetryShared::new(NoopFailureTelemetry));
  let ctx = TelemetryContext::new(None, extensions);

  let telemetry = builder.build(&ctx);
  telemetry.with_ref(|_impl| {});
}
