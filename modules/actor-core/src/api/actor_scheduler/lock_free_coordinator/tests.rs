use super::*;

#[test]
fn test_register_and_drain() {
  let mut coord = LockFreeCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.register_ready(idx);

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[test]
fn test_duplicate_detection() {
  let mut coord = LockFreeCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  // Register same index multiple times
  coord.register_ready(idx);
  coord.register_ready(idx);
  coord.register_ready(idx);

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  // Should only appear once
  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[test]
fn test_unregister() {
  let mut coord = LockFreeCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.register_ready(idx);
  coord.unregister(idx);

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  // Should be empty after unregister
  assert_eq!(out.len(), 0);
}

#[test]
fn test_handle_invoke_result_ready() {
  let mut coord = LockFreeCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.handle_invoke_result(idx, InvokeResult::Completed { ready_hint: true });

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[test]
fn test_handle_invoke_result_not_ready() {
  let mut coord = LockFreeCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.register_ready(idx);
  coord.handle_invoke_result(idx, InvokeResult::Completed { ready_hint: false });

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  // Should be empty after unregister
  assert_eq!(out.len(), 0);
}

#[test]
fn test_signal_notification() {
  let mut coord = LockFreeCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.register_ready(idx);

  // Signal should be pending
  use core::task::{Context, RawWaker, RawWakerVTable, Waker};
  const VTABLE: RawWakerVTable =
    RawWakerVTable::new(|_| RawWaker::new(std::ptr::null(), &VTABLE), |_| {}, |_| {}, |_| {});
  let raw_waker = RawWaker::new(std::ptr::null(), &VTABLE);
  let waker = unsafe { Waker::from_raw(raw_waker) };
  let mut cx = Context::from_waker(&waker);

  assert_eq!(coord.poll_wait_signal(&mut cx), Poll::Ready(()));

  // Second poll should be pending
  assert_eq!(coord.poll_wait_signal(&mut cx), Poll::Pending);
}

#[test]
fn test_concurrent_register() {
  use std::thread;

  let coord = LockFreeCoordinator::new(32);
  let coord_clone = coord.clone();

  let handles: Vec<_> = (0..8)
    .map(|thread_id| {
      let mut coord = coord_clone.clone();
      thread::spawn(move || {
        for i in 0..1000 {
          let idx = MailboxIndex::new((thread_id * 1000 + i) as u32, 0);
          coord.register_ready(idx);
        }
      })
    })
    .collect();

  for handle in handles {
    handle.join().unwrap();
  }

  // Drain all items
  let mut total = 0;
  let mut out = Vec::with_capacity(1000);
  let mut coord_mut = coord;

  loop {
    coord_mut.drain_ready_cycle(1000, &mut out);
    if out.is_empty() {
      break;
    }
    total += out.len();
    out.clear();
  }

  // Should have all 8000 items (8 threads * 1000 items)
  assert_eq!(total, 8000);
}
