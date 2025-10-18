use futures::{executor::block_on, join};

use super::ArcLocalAsyncBarrier;

#[test]
fn barrier_releases_all() {
  block_on(async {
    let barrier = ArcLocalAsyncBarrier::new(2);
    let other = barrier.clone();

    let first = barrier.wait();
    let second = other.wait();

    join!(first, second);
  });
}
