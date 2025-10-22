//! Tests for ReadyQueueCoordinator

use alloc::string::ToString;
use core::time::Duration;

use super::*;
#[cfg(feature = "std")]
use crate::api::actor_scheduler::default_ready_queue_coordinator::DefaultReadyQueueCoordinator;

#[cfg(feature = "std")]
#[test]
fn test_register_ready_basic() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.register_ready(idx);

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[cfg(feature = "std")]
#[test]
fn test_register_ready_duplicate_prevention() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  // Register same index twice
  coordinator.register_ready(idx);
  coordinator.register_ready(idx);

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should only appear once
  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[cfg(feature = "std")]
#[test]
fn test_drain_ready_cycle_batch_limit() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);

  // Register 5 mailboxes
  for i in 0..5 {
    coordinator.register_ready(MailboxIndex::new(i, 0));
  }

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(3, &mut out);

  // Should only drain 3
  assert_eq!(out.len(), 3);

  out.clear();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should drain remaining 2
  assert_eq!(out.len(), 2);
}

#[cfg(feature = "std")]
#[test]
fn test_unregister() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx1 = MailboxIndex::new(1, 0);
  let idx2 = MailboxIndex::new(2, 0);

  coordinator.register_ready(idx1);
  coordinator.register_ready(idx2);
  coordinator.unregister(idx1);

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should only contain idx2
  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx2);
}

#[cfg(feature = "std")]
#[test]
fn test_handle_invoke_result_completed_with_ready_hint() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.handle_invoke_result(idx, InvokeResult::Completed { ready_hint: true });

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should be re-registered
  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[cfg(feature = "std")]
#[test]
fn test_handle_invoke_result_completed_without_ready_hint() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.register_ready(idx);
  coordinator.handle_invoke_result(idx, InvokeResult::Completed { ready_hint: false });

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should not be in queue (already drained or unregistered)
  assert_eq!(out.len(), 0);
}

#[cfg(feature = "std")]
#[test]
fn test_handle_invoke_result_yielded() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.handle_invoke_result(idx, InvokeResult::Yielded);

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should be re-registered
  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[cfg(feature = "std")]
#[test]
fn test_handle_invoke_result_suspended() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.register_ready(idx);
  coordinator.handle_invoke_result(idx, InvokeResult::Suspended {
    reason:    SuspendReason::Backpressure,
    resume_on: ResumeCondition::WhenCapacityAvailable,
  });

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should be unregistered
  assert_eq!(out.len(), 0);
}

#[cfg(feature = "std")]
#[test]
fn test_handle_invoke_result_stopped() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.register_ready(idx);
  coordinator.handle_invoke_result(idx, InvokeResult::Stopped);

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);

  // Should be unregistered
  assert_eq!(out.len(), 0);
}

#[cfg(feature = "std")]
#[test]
fn test_throughput_hint() {
  let coordinator = DefaultReadyQueueCoordinator::new(64);
  assert_eq!(coordinator.throughput_hint(), 64);
}

#[test]
fn test_mailbox_index_equality() {
  let idx1 = MailboxIndex::new(1, 0);
  let idx2 = MailboxIndex::new(1, 0);
  let idx3 = MailboxIndex::new(1, 1);

  assert_eq!(idx1, idx2);
  assert_ne!(idx1, idx3);
}

#[test]
fn test_invoke_result_variants() {
  let result1 = InvokeResult::Completed { ready_hint: true };
  let result2 = InvokeResult::Yielded;
  let result3 = InvokeResult::Suspended {
    reason:    SuspendReason::Backpressure,
    resume_on: ResumeCondition::After(Duration::from_secs(1)),
  };
  let result4 = InvokeResult::Failed { error: "test error".to_string(), retry_after: None };
  let result5 = InvokeResult::Stopped;

  // Just verify they can be constructed
  assert!(matches!(result1, InvokeResult::Completed { ready_hint: true }));
  assert!(matches!(result2, InvokeResult::Yielded));
  assert!(matches!(result3, InvokeResult::Suspended { .. }));
  assert!(matches!(result4, InvokeResult::Failed { .. }));
  assert!(matches!(result5, InvokeResult::Stopped));
}

#[test]
fn test_mailbox_options_default() {
  let options = MailboxOptions::default();
  assert_eq!(options.capacity.get(), 1000);
  assert_eq!(options.overflow, OverflowStrategy::DropOldest);
  assert_eq!(options.reserve_for_system, 10);
}
