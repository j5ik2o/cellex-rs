use alloc::borrow::Cow;
use alloc::string::String;
use core::any::Any;
use core::fmt;
use core::ptr;

use cellex_utils_core_rs::sync::ArcShared;

#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;

/// Supervisor 向けに提供されるエラー情報の抽象化。
pub trait BehaviorFailure: fmt::Debug + Send + Sync + 'static {
  /// 任意型へのダウンキャストを可能にするためのフック。
  fn as_any(&self) -> &dyn Any;

  /// ログや表示用の説明。
  fn description(&self) -> Cow<'_, str> {
    Cow::Owned(format!("{:?}", self))
  }
}

/// デフォルトで利用される `BehaviorFailure` 実装。
#[derive(Clone, Debug)]
pub struct DefaultBehaviorFailure {
  message: Cow<'static, str>,
  debug: Option<String>,
}

impl DefaultBehaviorFailure {
  /// メッセージのみで失敗情報を構築する。
  #[must_use]
  pub fn from_message(message: impl Into<Cow<'static, str>>) -> Self {
    Self {
      message: message.into(),
      debug: None,
    }
  }

  /// 任意のエラー値から失敗情報を構築する。
  #[must_use]
  pub fn from_error<E>(error: E) -> Self
  where
    E: fmt::Display + fmt::Debug, {
    Self {
      message: Cow::Owned(alloc::format!("{error}")),
      debug: Some(alloc::format!("{error:?}")),
    }
  }

  /// panic payload の種類を判別できなかった場合に使用するフォールバック。
  #[must_use]
  pub fn from_unknown_panic(payload: &(dyn Any + Send)) -> Self {
    Self {
      message: Cow::Owned("panic: unknown payload".to_string()),
      debug: Some(alloc::format!("panic payload type_id: {:?}", payload.type_id())),
    }
  }

  /// デバッグ向けに詳細情報を取得する。
  #[must_use]
  pub fn debug_details(&self) -> Option<&str> {
    self.debug.as_deref()
  }
}

impl BehaviorFailure for DefaultBehaviorFailure {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn description(&self) -> Cow<'_, str> {
    self.message.clone()
  }
}

impl fmt::Display for DefaultBehaviorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.message.as_ref())
  }
}

/// アクターの失敗情報。`BehaviorFailure` をラップして Supervisor へ提供する。
#[derive(Clone)]
pub struct ActorFailure {
  inner: ArcShared<dyn BehaviorFailure>,
}

impl ActorFailure {
  /// `BehaviorFailure` をラップして `ActorFailure` を生成する。
  #[must_use]
  pub fn new(inner: impl BehaviorFailure) -> Self {
    let boxed: Box<dyn BehaviorFailure> = Box::new(inner);
    let arc: Arc<dyn BehaviorFailure> = boxed.into();
    Self {
      inner: ArcShared::from_arc(arc),
    }
  }

  /// 既存の共有参照から `ActorFailure` を生成する。
  #[must_use]
  pub fn from_shared(inner: ArcShared<dyn BehaviorFailure>) -> Self {
    Self { inner }
  }

  /// 文字列メッセージから失敗情報を生成する。
  #[must_use]
  pub fn from_message(message: impl Into<Cow<'static, str>>) -> Self {
    Self::new(DefaultBehaviorFailure::from_message(message))
  }

  /// 任意のエラー値から失敗情報を生成する。
  #[must_use]
  pub fn from_error<E>(error: E) -> Self
  where
    E: fmt::Display + fmt::Debug, {
    Self::new(DefaultBehaviorFailure::from_error(error))
  }

  /// panic 由来のペイロードを失敗情報へ変換する。
  #[must_use]
  pub fn from_panic_payload(payload: &(dyn Any + Send)) -> Self {
    if let Some(failure) = payload.downcast_ref::<ActorFailure>() {
      return failure.clone();
    }

    if let Some(default) = payload.downcast_ref::<DefaultBehaviorFailure>() {
      return Self::new(default.clone());
    }

    if let Some(message) = payload.downcast_ref::<&str>() {
      return Self::from_message(alloc::format!("panic: {message}"));
    }

    if let Some(message) = payload.downcast_ref::<String>() {
      return Self::from_message(alloc::format!("panic: {message}"));
    }

    Self::new(DefaultBehaviorFailure::from_unknown_panic(payload))
  }

  /// `BehaviorFailure` への参照を取得する。
  #[must_use]
  pub fn behavior(&self) -> &dyn BehaviorFailure {
    &*self.inner
  }

  /// 表示用の説明を取得する。
  #[must_use]
  pub fn description(&self) -> Cow<'_, str> {
    self.inner.description()
  }
}

impl fmt::Debug for ActorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.behavior(), f)
  }
}

impl fmt::Display for ActorFailure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let description = self.description();
    f.write_str(description.as_ref())
  }
}

impl<T> From<T> for ActorFailure
where
  T: BehaviorFailure,
{
  fn from(value: T) -> Self {
    Self::new(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct SampleError;

  impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      f.write_str("sample error")
    }
  }

  #[test]
  fn actor_failure_from_error_exposes_behavior_failure() {
    let failure = ActorFailure::from_error(SampleError);

    let description = failure.description();
    assert!(description.contains("sample error"));

    let inner = failure
      .behavior()
      .as_any()
      .downcast_ref::<DefaultBehaviorFailure>()
      .expect("default failure");
    assert_eq!(inner.description(), description);
    assert!(inner.debug_details().unwrap().contains("SampleError"));
  }

  #[test]
  fn actor_failure_from_panic_payload_formats_message() {
    let payload = Box::new("boom");
    let failure = ActorFailure::from_panic_payload(payload.as_ref());
    assert!(failure.description().contains("panic: boom"));

    let string_payload = Box::new(String::from("kapow"));
    let failure = ActorFailure::from_panic_payload(string_payload.as_ref());
    assert!(failure.description().contains("panic: kapow"));
  }

  #[test]
  fn actor_failure_unknown_panic_uses_fallback() {
    let payload = Box::new(42_u32);
    let failure = ActorFailure::from_panic_payload(payload.as_ref());
    assert!(failure.description().contains("panic: unknown payload"));
  }
}

impl PartialEq for ActorFailure {
  fn eq(&self, other: &Self) -> bool {
    if ptr::eq(self.behavior(), other.behavior()) {
      return true;
    }
    self.description() == other.description()
  }
}

impl Eq for ActorFailure {}
