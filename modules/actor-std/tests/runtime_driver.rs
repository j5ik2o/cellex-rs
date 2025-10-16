use std::sync::{Arc, Mutex};

use cellex_actor_core_rs::{ActorSystem, ActorSystemConfig, FailureEventStream, Props};
use cellex_actor_std_rs::{tokio_actor_runtime, FailureEventHub, TokioActorRuntime, TokioSystemHandle};
use core::num::NonZeroUsize;

async fn run_tokio_actor_runtime_processes_messages(worker_count: NonZeroUsize) {
  let failure_hub = FailureEventHub::new();
  let actor_runtime: TokioActorRuntime = tokio_actor_runtime();
  let config = ActorSystemConfig::default()
    .with_failure_event_listener(Some(failure_hub.listener()))
    .with_ready_queue_worker_count(Some(worker_count));
  let mut system: ActorSystem<u32, _> = ActorSystem::new_with_actor_runtime(actor_runtime, config);

  let state: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
  let state_clone = state.clone();

  let props = Props::new(move |_, msg: u32| {
    state_clone.lock().unwrap().push(msg);
    Ok(())
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");

  actor_ref.tell(7).expect("tell");
  system.run_until_idle().expect("run until idle");

  assert_eq!(state.lock().unwrap().as_slice(), &[7]);

  let runner = system.into_runner();
  assert_eq!(runner.ready_queue_worker_count(), worker_count);
}

#[tokio::test(flavor = "current_thread")]
async fn tokio_actor_runtime_processes_messages() {
  let worker_count = NonZeroUsize::new(1).expect("non-zero worker count");
  run_tokio_actor_runtime_processes_messages(worker_count).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_actor_runtime_processes_messages_multi_thread() {
  let worker_count = NonZeroUsize::new(2).expect("non-zero worker count");
  run_tokio_actor_runtime_processes_messages(worker_count).await;
}

async fn run_tokio_system_handle_can_be_aborted(worker_count: NonZeroUsize) {
  tokio::task::LocalSet::new()
    .run_until(async move {
      let failure_hub = FailureEventHub::new();
      let actor_runtime: TokioActorRuntime = tokio_actor_runtime();
      let config = ActorSystemConfig::default()
        .with_failure_event_listener(Some(failure_hub.listener()))
        .with_ready_queue_worker_count(Some(worker_count));

      let runner = {
        let system: ActorSystem<u32, _> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
        let runner = system.into_runner();
        assert_eq!(runner.ready_queue_worker_count(), worker_count);
        runner
      };

      let handle: TokioSystemHandle<u32> = TokioSystemHandle::start_local(runner);
      let listener = handle.spawn_ctrl_c_listener();
      handle.abort();
      listener.abort();
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn tokio_system_handle_can_be_aborted() {
  let worker_count = NonZeroUsize::new(2).expect("non-zero worker count");
  run_tokio_system_handle_can_be_aborted(worker_count).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_system_handle_can_be_aborted_multi_thread() {
  let worker_count = NonZeroUsize::new(2).expect("non-zero worker count");
  run_tokio_system_handle_can_be_aborted(worker_count).await;
}
