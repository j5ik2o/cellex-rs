use super::ArcLocalWaitGroup;
use futures::executor::block_on;
use futures::join;

#[test]
fn wait_group_completes() {
  block_on(async {
    let wg = ArcLocalWaitGroup::new();
    wg.add(2);
    let clone = wg.clone();
    let worker = async move {
      clone.done();
      clone.done();
    };
    join!(worker, wg.wait());
  });
}
