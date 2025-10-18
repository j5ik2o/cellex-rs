use tokio::join;

use super::AsyncBarrier;

#[tokio::test]
async fn barrier_releases_all() {
  let barrier = AsyncBarrier::new(2);
  let b2 = barrier.clone();

  let first = async move {
    barrier.wait().await;
  };
  let second = async move {
    b2.wait().await;
  };

  join!(first, second);
}
