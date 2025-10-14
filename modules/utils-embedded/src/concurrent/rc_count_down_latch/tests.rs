use super::CountDownLatch;
use futures::executor::block_on;
use futures::join;

#[test]
fn latch_reaches_zero() {
  block_on(async {
    let latch = CountDownLatch::new(2);
    let clone = latch.clone();

    let wait_fut = latch.wait();
    let worker = async move {
      clone.count_down().await;
      clone.count_down().await;
    };

    join!(worker, wait_fut);
  });
}
