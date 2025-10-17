use cellex_actor_core_rs::api::actor_system::Spawn;
use core::future::Future;

/// Shared spawn adapter built on top of `tokio::spawn`.
pub struct TokioSpawner;

impl Spawn for TokioSpawner {
  fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
  }
}
