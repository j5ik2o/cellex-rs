#[cfg(feature = "embassy_executor")]
mod sample {
  use cellex_actor_core_rs::{ActorSystem, ActorSystemConfig, Props};
  use cellex_actor_embedded_rs::{define_embassy_dispatcher, embassy_actor_runtime, EmbassyActorRuntime};
  use core::sync::atomic::{AtomicU32, Ordering};
  use embassy_executor::Executor;
  use static_cell::StaticCell;

  static EXECUTOR: StaticCell<Executor> = StaticCell::new();
  static SYSTEM: StaticCell<ActorSystem<u32, EmbassyActorRuntime, _>> = StaticCell::new();
  pub static MESSAGE_SUM: AtomicU32 = AtomicU32::new(0);

  define_embassy_dispatcher!(
    pub fn dispatcher(system: ActorSystem<u32, EmbassyActorRuntime, _>)
  );

  pub fn run() {
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
      let actor_runtime = embassy_actor_runtime(spawner);
      let system =
        SYSTEM.init_with(|| ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default()));

      {
        let mut root = system.root_context();
        let actor_ref = root
          .spawn(Props::new(|_, msg: u32| {
            MESSAGE_SUM.fetch_add(msg, Ordering::Relaxed);
            Ok(())
          }))
          .expect("spawn actor");
        actor_ref.tell(1).expect("tell");
        actor_ref.tell(2).expect("tell");
      }

      spawner.spawn(dispatcher(system)).expect("spawn dispatcher");
    });
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
