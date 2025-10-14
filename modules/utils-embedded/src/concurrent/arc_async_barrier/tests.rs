use super::ArcLocalAsyncBarrier;
use futures::executor::block_on;
use futures::join;

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
