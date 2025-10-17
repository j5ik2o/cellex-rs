use alloc::borrow::Cow;

/// Telemetry に渡されるタグ（キー／バリュー）ペア。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryTag {
  key: Cow<'static, str>,
  value: Cow<'static, str>,
}

impl TelemetryTag {
  /// 新しいタグを生成する。
  #[must_use]
  pub fn new(key: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>) -> Self {
    Self {
      key: key.into(),
      value: value.into(),
    }
  }

  /// タグのキーを返す。
  #[must_use]
  pub fn key(&self) -> &str {
    self.key.as_ref()
  }

  /// タグの値を返す。
  #[must_use]
  pub fn value(&self) -> &str {
    self.value.as_ref()
  }
}
