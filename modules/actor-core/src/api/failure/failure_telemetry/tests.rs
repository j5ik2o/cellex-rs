#![allow(clippy::disallowed_types)]

use super::{FailureTelemetryBuilderShared, FailureTelemetryContext, FailureTelemetryShared, NoopFailureTelemetry};
use crate::api::extensions::Extensions;

#[test]
fn telemetry_builder_shared_invokes_closure() {
  let extensions = Extensions::new();
  let builder = FailureTelemetryBuilderShared::new(|_ctx| FailureTelemetryShared::new(NoopFailureTelemetry));
  let ctx = FailureTelemetryContext::new(None, extensions);

  let telemetry = builder.build(&ctx);
  telemetry.with_ref(|_impl| {});
}
