extern crate alloc;

use alloc::vec;

use futures::executor::block_on;

use super::{Synchronized, SynchronizedRw};

#[test]
fn rc_mutex_backend_basic() {
  block_on(async {
    let sync = Synchronized::new(1);
    let value = sync.read(|guard| **guard).await;
    assert_eq!(value, 1);

    sync
      .write(|guard| {
        **guard = 7;
      })
      .await;

    let updated = {
      let guard = sync.lock().await;
      let guard = guard.into_inner();
      *guard
    };
    assert_eq!(updated, 7);
  });
}

#[test]
fn rc_rw_backend_behaviour() {
  block_on(async {
    let sync = SynchronizedRw::new(vec![1]);
    let len = sync.read(|guard| guard.len()).await;
    assert_eq!(len, 1);

    sync
      .write(|guard| {
        guard.push(2);
      })
      .await;

    let sum = {
      let guard = sync.read_guard().await;
      guard.iter().sum::<i32>()
    };
    assert_eq!(sum, 3);
  });
}
