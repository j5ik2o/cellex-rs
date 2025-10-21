use alloc::borrow::Cow;

/// Key/value pair attached to telemetry events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryTag {
  key:   Cow<'static, str>,
  value: Cow<'static, str>,
}

impl TelemetryTag {
  /// Creates a new telemetry tag.
  #[must_use]
  pub fn new(key: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>) -> Self {
    Self { key: key.into(), value: value.into() }
  }

  /// Returns the tag key.
  #[must_use]
  pub fn key(&self) -> &str {
    self.key.as_ref()
  }

  /// Returns the tag value.
  #[must_use]
  pub fn value(&self) -> &str {
    self.value.as_ref()
  }
}
