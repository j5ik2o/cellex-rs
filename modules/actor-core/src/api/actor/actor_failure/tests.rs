use super::*;

use ::core::fmt;
use alloc::boxed::Box;
use alloc::string::String;

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
