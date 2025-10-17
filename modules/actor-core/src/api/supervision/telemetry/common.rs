use super::telemetry_tag::TelemetryTag;
#[cfg(all(feature = "std", feature = "unwind-supervision"))]
use super::tracing_failure_telemetry::tracing_failure_telemetry;
use crate::api::failure_telemetry::FailureTelemetryShared;
use crate::api::supervision::failure::FailureMetadata;
use alloc::borrow::Cow;
use alloc::vec::Vec;

/// `FailureSnapshot` が保持するタグ数の上限。
pub const MAX_FAILURE_SNAPSHOT_TAGS: usize = 8;

/// Returns the default telemetry implementation for the current build configuration.
pub fn default_failure_telemetry_shared() -> FailureTelemetryShared {
  #[cfg(all(feature = "std", feature = "unwind-supervision"))]
  {
    return tracing_failure_telemetry();
  }

  #[cfg(not(all(feature = "std", feature = "unwind-supervision")))]
  {
    return super::noop_failure_telemetry::noop_failure_telemetry_shared();
  }
}

pub(crate) fn build_snapshot_tags(metadata: &FailureMetadata) -> Vec<TelemetryTag> {
  let mut tags = Vec::new();

  if let Some(component) = metadata.component.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(TelemetryTag::new(
        Cow::Borrowed("component"),
        Cow::Owned(component.clone()),
      ));
    }
  }
  if let Some(endpoint) = metadata.endpoint.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(TelemetryTag::new(
        Cow::Borrowed("endpoint"),
        Cow::Owned(endpoint.clone()),
      ));
    }
  }
  if let Some(transport) = metadata.transport.as_ref() {
    if tags.len() < MAX_FAILURE_SNAPSHOT_TAGS {
      tags.push(TelemetryTag::new(
        Cow::Borrowed("transport"),
        Cow::Owned(transport.clone()),
      ));
    }
  }

  for (key, value) in metadata.tags.iter() {
    if tags.len() >= MAX_FAILURE_SNAPSHOT_TAGS {
      break;
    }
    tags.push(TelemetryTag::new(Cow::Owned(key.clone()), Cow::Owned(value.clone())));
  }

  tags
}
