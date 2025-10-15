#[cfg(feature = "embassy_executor")]
mod sample {
  use cellex_actor_core_rs::{
    actor::context::RootContext, ActorSystem, ActorSystemConfig, Props, ReadyQueueWorker, RuntimeEnv, ShutdownToken,
  };
  use cellex_actor_core_rs::{ArcShared, DynMessage};
  use cellex_actor_embedded_rs::{ActorRuntimeBundleEmbassyExt, LocalMailboxRuntime};
  use core::num::NonZeroUsize;
  use core::sync::atomic::{AtomicU32, Ordering};
  use embassy_executor::{Executor, Spawner};
  use embassy_futures::select::{select, Either};
  use embassy_futures::{pin_mut, yield_now};
  use embassy_time::Timer;
  use static_cell::StaticCell;

  static EXECUTOR: StaticCell<Executor> = StaticCell::new();
  static SYSTEM: StaticCell<ActorSystem<u32, RuntimeEnv<LocalMailboxRuntime>>> = StaticCell::new();
  static MESSAGE_SUM: AtomicU32 = AtomicU32::new(0);

  #[embassy_executor::task]
  async fn ready_queue_worker_task(
    worker: ArcShared<dyn ReadyQueueWorker<DynMessage, RuntimeEnv<LocalMailboxRuntime>>>,
    shutdown: ShutdownToken,
  ) {
    loop {
      if shutdown.is_triggered() {
        return;
      }

      match worker.process_ready_once() {
        Ok(Some(true)) | Ok(Some(false)) => {
          yield_now().await;
          continue;
        }
        Ok(None) => {}
        Err(err) => panic!("ready queue processing failed: {:?}", err),
      }

      match worker.wait_for_ready() {
        Some(wait_future) => {
          pin_mut!(wait_future);
          let shutdown_future = wait_for_shutdown_signal(shutdown.clone());
          pin_mut!(shutdown_future);
          match select(wait_future, shutdown_future).await {
            Either::First((_, _)) => {}
            Either::Second((_, _)) => return,
          }
        }
        None => yield_now().await,
      }
    }
  }

  fn spawn_ready_queue_workers(
    spawner: &Spawner,
    worker: ArcShared<dyn ReadyQueueWorker<DynMessage, RuntimeEnv<LocalMailboxRuntime>>>,
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
      let runtime = RuntimeEnv::new(LocalMailboxRuntime::default()).with_embassy_scheduler(spawner);
      let system = SYSTEM.init_with(|| ActorSystem::new_with_runtime(runtime, ActorSystemConfig::default()));
      let shutdown = system.shutdown_token();

      let mut root: RootContext<'_, u32, _> = system.root_context();
      let actor_ref = root
        .spawn(Props::new(|_, msg: u32| {
          MESSAGE_SUM.fetch_add(msg, Ordering::Relaxed);
          Ok(())
        }))
        .expect("spawn actor");

      let worker = system.ready_queue_worker().expect("ready queue worker");
      spawn_ready_queue_workers(spawner, worker, shutdown.clone(), NonZeroUsize::new(3).unwrap());

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
