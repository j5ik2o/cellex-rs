//! Tests for runtime mutex factory functionality

use cellex_utils_core_rs::sync::sync_mutex_like::SyncMutexLike;

use crate::api::{
  actor_runtime::{ActorRuntime, GenericActorRuntime},
  test_support::TestMailboxFactory,
};

#[test]
fn test_sync_mutex_factory() {
  let runtime = GenericActorRuntime::new(TestMailboxFactory::default());
  let factory = runtime.sync_mutex_factory::<i32>();

  // Use the factory to create a mutex
  let mutex = factory(42);
  assert_eq!(*mutex.lock(), 42);

  // Verify the mutex works correctly
  {
    let mut guard = mutex.lock();
    *guard = 100;
  }
  assert_eq!(*mutex.lock(), 100);
}

#[cfg(feature = "std")]
#[tokio::test]
async fn test_async_mutex_factory() {
  use cellex_utils_core_rs::sync::async_mutex_like::AsyncMutexLike;

  let runtime = GenericActorRuntime::new(TestMailboxFactory::default());
  let factory = runtime.async_mutex_factory::<i32>();

  // Use the factory to create an async mutex
  let mutex = factory(42);
  assert_eq!(*mutex.lock().await, 42);

  // Verify the mutex works correctly
  {
    let mut guard = mutex.lock().await;
    *guard = 100;
  }
  assert_eq!(*mutex.lock().await, 100);
}

#[test]
fn test_factory_can_be_cloned() {
  let runtime = GenericActorRuntime::new(TestMailboxFactory::default());
  let factory1 = runtime.sync_mutex_factory::<String>();
  let factory2 = factory1.clone();

  let mutex1 = factory1("hello".to_string());
  let mutex2 = factory2("world".to_string());

  assert_eq!(*mutex1.lock(), "hello");
  assert_eq!(*mutex2.lock(), "world");
}
