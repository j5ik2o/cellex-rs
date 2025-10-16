//! ReadyQueue ワーカを 4 本起動し、Tokio 上でメッセージを並列処理するサンプル。

use cellex_actor_core_rs::{ActorSystem, ActorSystemConfig, GenericActorRuntime, Props};
use cellex_actor_std_rs::{TokioMailboxRuntime, TokioSystemHandle};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::task::LocalSet;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  let local = LocalSet::new();
  local
    .run_until(async move {
      let mut system: ActorSystem<u32, _> = ActorSystem::new_with_runtime(
        GenericActorRuntime::new(TokioMailboxRuntime),
        ActorSystemConfig::default(),
      );
      let shutdown = system.shutdown_token();
      let mut root = system.root_context();

      let log = Arc::new(Mutex::new(Vec::new()));
      let log_clone = Arc::clone(&log);

      let props = Props::new(move |_, msg: u32| {
        println!("Received message: {}", msg);
        log_clone.lock().unwrap().push(msg);
        Ok(())
      });
      let actor_ref = root.spawn(props).expect("spawn actor");

      let worker_count = NonZeroUsize::new(4).expect("worker count");
      let runner = system.into_runner().with_ready_queue_worker_count(worker_count);
      let handle = TokioSystemHandle::start_local(runner);

      for value in 0..32u32 {
        actor_ref.tell(value).expect("tell");
      }

      tokio::time::sleep(std::time::Duration::from_millis(20)).await;
      shutdown.trigger();
      let _ = handle.await_terminated().await;

      let mut entries = log.lock().unwrap().clone();
      entries.sort();
      assert_eq!(entries, (0..32).collect::<Vec<_>>());
    })
    .await;
}
