#![allow(clippy::disallowed_types)]
use futures::{executor::block_on, join};

use super::WaitGroup;

#[test]
fn wait_group_completes() {
  block_on(async {
    let wg = WaitGroup::new();
    wg.add(2);
    let clone = wg.clone();
    let worker = async move {
      clone.done();
      clone.done();
    };
    join!(worker, wg.wait());
  });
}
