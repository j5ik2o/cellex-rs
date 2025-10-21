use alloc::{borrow::Cow, vec::Vec};

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
use crate::api::failure::failure_telemetry::tracing_failure_telemetry::tracing_failure_telemetry;
use crate::api::failure::{
  failure_telemetry::{FailureTelemetryShared, FailureTelemetryTag},
  FailureMetadata,
};

/// Maximum number of tags stored inside a `FailureSnapshot`.
pub const MAX_FAILURE_SNAPSHOT_TAGS: usize = 8;

/// Returns the default telemetry implementation for the current build configuration.
#[must_use]
pub fn default_failure_telemetry_shared() -> FailureTelemetryShared {
  #[cfg(all(feature = "std", feature = "unwind-supervision"))]
  {
    tracing_failure_telemetry()
  }

  #[cfg(not(all(feature = "std", feature = "unwind-supervision")))]
  {
    crate::api::failure::failure_telemetry::noop_failure_telemetry_shared()
  }
}

pub(crate) fn build_snapshot_tags(metadata: &FailureMetadata) -> Vec<FailureTelemetryTag> {
  let mut tags = Vec::new();

  if let Some(component) = metadata.component.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(FailureTelemetryTag::new(Cow::Borrowed("component"), Cow::Owned(component.clone())));
    }
  }
  if let Some(endpoint) = metadata.endpoint.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(FailureTelemetryTag::new(Cow::Borrowed("endpoint"), Cow::Owned(endpoint.clone())));
    }
  }
  if let Some(transport) = metadata.transport.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(FailureTelemetryTag::new(Cow::Borrowed("transport"), Cow::Owned(transport.clone())));
    }
  }

  for (key, value) in metadata.tags.iter() {
    if tags.len() >= MAX_FAILURE_SNAPSHOT_TAGS {
      break;
    }
    tags.push(FailureTelemetryTag::new(Cow::Owned(key.clone()), Cow::Owned(value.clone())));
  }

  tags
}
