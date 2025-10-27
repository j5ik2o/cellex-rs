use core::future::Future;

use cellex_actor_core_rs::api::actor::Spawn;

/// Shared spawn adapter built on top of `tokio::spawn`.
pub struct TokioSpawner;

impl Spawn for TokioSpawner {
  fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
  }
}
