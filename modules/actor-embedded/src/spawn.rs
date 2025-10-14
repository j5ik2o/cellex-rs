#[cfg(test)]
mod tests;

use core::future::Future;

use cellex_actor_core_rs::Spawn;

/// A spawner that immediately drops futures.
///
/// An implementation for embedded environments that simply drops tasks without actually executing them.
pub struct ImmediateSpawner;

impl Spawn for ImmediateSpawner {
  fn spawn(&self, _fut: impl Future<Output = ()> + 'static) {}
}


