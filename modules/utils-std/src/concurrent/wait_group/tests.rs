use tokio::join;

use super::WaitGroup;

#[tokio::test]
async fn wait_group_completes() {
  let wg = WaitGroup::new();
  wg.add(2);
  let worker_wg = wg.clone();

  let wait_fut = wg.wait();
  let worker = async move {
    worker_wg.done();
    worker_wg.done();
  };

  join!(worker, wait_fut);
}
