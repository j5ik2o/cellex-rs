extern crate alloc;

use alloc::vec;

use futures::executor::block_on;

use super::{ArcLocalSynchronized, ArcLocalSynchronizedRw};

#[test]
fn arc_mutex_backend_behaviour() {
  block_on(async {
    let sync = ArcLocalSynchronized::new(10);
    let value = sync.read(|guard| **guard).await;
    assert_eq!(value, 10);

    sync
      .write(|guard| {
        **guard = 20;
      })
      .await;

    let updated = {
      let guard = sync.lock().await;
      let guard = guard.into_inner();
      *guard
    };
    assert_eq!(updated, 20);
  });
}

#[test]
fn arc_rw_backend_behaviour() {
  block_on(async {
    let sync = ArcLocalSynchronizedRw::new(vec![1, 2]);
    let len = sync.read(|guard| guard.len()).await;
    assert_eq!(len, 2);

    sync
      .write(|guard| {
        guard.push(3);
      })
      .await;

    let sum = {
      let guard = sync.read_guard().await;
      guard.iter().copied().sum::<i32>()
    };
    assert_eq!(sum, 6);
  });
}
