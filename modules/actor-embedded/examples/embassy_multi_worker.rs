#[cfg(feature = "embassy_executor")]
mod sample {
  use cellex_actor_core_rs::{
    actor::context::RootContext, drive_ready_queue_worker, ActorSystem, ActorSystemConfig, MailboxOf, Props,
    ReadyQueueWorker, ShutdownToken,
  };
  use cellex_actor_core_rs::{ArcShared, DynMessage};
  use cellex_actor_embedded_rs::{embassy_actor_runtime, EmbassyActorRuntime};
  use core::num::NonZeroUsize;
  use core::sync::atomic::{AtomicU32, Ordering};
  use embassy_executor::{Executor, Spawner};
  use embassy_futures::yield_now;
  use embassy_time::Timer;
  use static_cell::StaticCell;

  static EXECUTOR: StaticCell<Executor> = StaticCell::new();
  static SYSTEM: StaticCell<ActorSystem<u32, EmbassyActorRuntime>> = StaticCell::new();
  static MESSAGE_SUM: AtomicU32 = AtomicU32::new(0);

  #[embassy_executor::task]
  async fn ready_queue_worker_task(
    worker: ArcShared<dyn ReadyQueueWorker<DynMessage, MailboxOf<EmbassyActorRuntime>>>,
    shutdown: ShutdownToken,
  ) {
    let mut shutdown_for_wait = shutdown.clone();
    drive_ready_queue_worker(
      worker,
      shutdown,
      || yield_now(),
      move || wait_for_shutdown_signal(shutdown_for_wait.clone()),
    )
    .await
    .expect("ready queue worker failed");
  }

  fn spawn_ready_queue_workers(
    spawner: &Spawner,
    worker: ArcShared<dyn ReadyQueueWorker<DynMessage, MailboxOf<EmbassyActorRuntime>>>,
    shutdown: ShutdownToken,
    worker_count: NonZeroUsize,
  ) {
    for _ in 0..worker_count.get() {
      spawner
        .spawn(ready_queue_worker_task(worker.clone(), shutdown.clone()))
        .expect("spawn worker task");
    }
  }

  async fn wait_for_shutdown_signal(token: ShutdownToken) {
    while !token.is_triggered() {
      yield_now().await;
    }
  }

  #[embassy_executor::task]
  async fn shutdown_listener(token: ShutdownToken) {
    wait_for_shutdown_signal(token).await;
  }

  pub fn run() {
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
      let configured_worker_count = NonZeroUsize::new(3).expect("non-zero worker count");
      let runtime = embassy_actor_runtime(spawner);
      let config = ActorSystemConfig::default().with_ready_queue_worker_count(Some(configured_worker_count));
      let system = SYSTEM.init_with(|| ActorSystem::new_with_runtime(runtime, config));
      let shutdown = system.shutdown_token();
      let worker_count = configured_worker_count;

      let mut root: RootContext<'_, u32, _> = system.root_context();
      let actor_ref = root
        .spawn(Props::new(|_, msg: u32| {
          MESSAGE_SUM.fetch_add(msg, Ordering::Relaxed);
          Ok(())
        }))
        .expect("spawn actor");

      let worker = system.ready_queue_worker().expect("ready queue worker");
      spawn_ready_queue_workers(spawner, worker, shutdown.clone(), worker_count);

      for value in 0..10 {
        actor_ref.tell(value).expect("tell");
      }

      spawner
        .spawn(shutdown_listener(shutdown.clone()))
        .expect("spawn shutdown wait");

      Timer::after_millis(10).await;
      shutdown.trigger();
    });

    assert_eq!(MESSAGE_SUM.load(Ordering::Relaxed), (0..10).sum());
  }
}

#[cfg(feature = "embassy_executor")]
fn main() {
  sample::run();
}

#[cfg(not(feature = "embassy_executor"))]
fn main() {
  panic!("Run with --features embassy_executor to build this example");
}
