use alloc::vec::Vec;

use super::*;

#[test]
fn test_adaptive_selection() {
  // All concurrency hints use the same implementation in no_std
  let coord_low = AdaptiveCoordinator::new(32, 2);
  let coord_boundary = AdaptiveCoordinator::new(32, 4);
  let coord_high = AdaptiveCoordinator::new(32, 8);

  // Verify throughput_hint works
  assert_eq!(coord_low.throughput_hint(), 32);
  assert_eq!(coord_boundary.throughput_hint(), 32);
  assert_eq!(coord_high.throughput_hint(), 32);
}

#[test]
fn test_explicit_strategy() {
  // Force locked (only available option)
  let coord_locked = AdaptiveCoordinator::with_strategy(32, false);
  assert_eq!(coord_locked.throughput_hint(), 32);
}

#[test]
#[should_panic(expected = "lock-free coordinators are only available in cellex-actor-std-rs")]
fn test_explicit_strategy_lockfree_panics() {
  // Should panic in no_std
  let _coord_lockfree = AdaptiveCoordinator::with_strategy(32, true);
}

#[test]
fn test_register_and_drain() {
  let coord = AdaptiveCoordinator::new(32, 2);
  let idx = MailboxIndex::new(1, 0);

  coord.register_ready(idx);

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}
