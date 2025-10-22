use super::*;
use std::sync::Arc;
use std::thread;

#[test]
fn test_register_and_drain() {
  let coord = LockFreeCoordinatorV2::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.register_ready(idx);

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}

#[test]
fn test_duplicate_detection() {
  let coord = LockFreeCoordinatorV2::new(32);
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
fn test_concurrent_register() {
  let coord = Arc::new(LockFreeCoordinatorV2::new(32));
  let items_per_thread = 1000;

  let handles: Vec<_> = (0..8)
    .map(|thread_id| {
      let coord_clone = Arc::clone(&coord);
      thread::spawn(move || {
        for i in 0..items_per_thread {
          let idx = MailboxIndex::new((thread_id * items_per_thread + i) as u32, 0);
          coord_clone.register_ready(idx);
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

  loop {
    coord.drain_ready_cycle(1000, &mut out);
    if out.is_empty() {
      break;
    }
    total += out.len();
    out.clear();
  }

  // Should have all 8000 items
  assert_eq!(total, 8000);
}

#[test]
fn test_concurrent_drain() {
  let coord = Arc::new(LockFreeCoordinatorV2::new(32));

  // Pre-populate queue
  for i in 0..10000 {
    coord.register_ready(MailboxIndex::new(i, 0));
  }

  // Multiple threads drain concurrently
  let handles: Vec<_> = (0..4)
    .map(|_| {
      let coord_clone = Arc::clone(&coord);
      thread::spawn(move || {
        let mut out = Vec::with_capacity(100);
        let mut count = 0;

        for _ in 0..30 {
          coord_clone.drain_ready_cycle(100, &mut out);
          count += out.len();
          out.clear();
        }

        count
      })
    })
    .collect();

  let mut total = 0;
  for handle in handles {
    total += handle.join().unwrap();
  }

  // Should drain all 10000 items (no duplicates across threads)
  assert_eq!(total, 10000);
}

#[test]
fn test_handle_invoke_result() {
  let coord = LockFreeCoordinatorV2::new(32);
  let idx = MailboxIndex::new(1, 0);

  coord.handle_invoke_result(idx, InvokeResult::Completed { ready_hint: true });

  let mut out = Vec::new();
  coord.drain_ready_cycle(10, &mut out);

  assert_eq!(out.len(), 1);
  assert_eq!(out[0], idx);
}
