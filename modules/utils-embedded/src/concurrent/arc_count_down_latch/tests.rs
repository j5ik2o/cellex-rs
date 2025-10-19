#![allow(clippy::disallowed_types)]
use futures::{executor::block_on, join};

use super::ArcLocalCountDownLatch;

#[test]
fn latch_waits_for_completion() {
  block_on(async {
    let latch = ArcLocalCountDownLatch::new(2);
    let worker_latch = latch.clone();

    let wait_fut = latch.wait();
    let worker = async move {
      worker_latch.count_down().await;
      worker_latch.count_down().await;
    };

    join!(worker, wait_fut);
  });
}
