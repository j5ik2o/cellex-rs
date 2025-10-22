use alloc::vec::Vec;

use super::*;

#[test]
fn test_adaptive_selection() {
  // Low concurrency → Locked
  let coord_low = AdaptiveCoordinator::new(32, 2);
  assert!(matches!(coord_low, AdaptiveCoordinator::Locked(_)));

  // Boundary case (4 threads) → Locked
  let coord_boundary = AdaptiveCoordinator::new(32, 4);
  assert!(matches!(coord_boundary, AdaptiveCoordinator::Locked(_)));

  // High concurrency → LockFree
  let coord_high = AdaptiveCoordinator::new(32, 8);
  assert!(matches!(coord_high, AdaptiveCoordinator::LockFree(_)));
}

#[test]
fn test_explicit_strategy() {
  // Force locked
  let coord_locked = AdaptiveCoordinator::with_strategy(32, false);
  assert!(matches!(coord_locked, AdaptiveCoordinator::Locked(_)));

  // Force lock-free
  let coord_lockfree = AdaptiveCoordinator::with_strategy(32, true);
  assert!(matches!(coord_lockfree, AdaptiveCoordinator::LockFree(_)));
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
