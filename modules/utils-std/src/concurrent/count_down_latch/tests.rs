use super::CountDownLatch;
use tokio::join;

#[tokio::test]
async fn latch_reaches_zero() {
  let latch = CountDownLatch::new(2);
  let latch_clone = latch.clone();
  let wait_fut = latch.wait();
  let worker = async move {
    latch_clone.count_down().await;
    latch_clone.count_down().await;
  };

  join!(worker, wait_fut);
}
