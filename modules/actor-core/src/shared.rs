mod failure_telemetry;
mod map_system;
mod receive_timeout;

pub use failure_telemetry::{
  FailureEventHandlerShared, FailureEventListenerShared, FailureTelemetryBuilderShared, FailureTelemetryShared,
  TelemetryContext,
};
pub use map_system::MapSystemShared;
pub use receive_timeout::{ReceiveTimeoutDriver, ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared};

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Extensions;
  use crate::NoopFailureTelemetry;

  #[test]
  fn telemetry_builder_shared_invokes_closure() {
    let extensions = Extensions::new();
    let builder = FailureTelemetryBuilderShared::new(|_ctx| FailureTelemetryShared::new(NoopFailureTelemetry));
    let ctx = TelemetryContext::new(None, extensions);

    let telemetry = builder.build(&ctx);
    telemetry.with_ref(|_impl| {});
  }
}
