#![cfg(feature = "embassy_executor")]

/// Macro that defines a dispatcher task for Embassy executors.
///
/// # Usage
/// ```rust
/// use cellex_actor_embedded_rs::{define_embassy_dispatcher, LocalMailboxRuntime};
/// use cellex_actor_core_rs::{GenericActorRuntime, ActorSystem, ActorSystemConfig};
/// use embassy_executor::Spawner;
///
/// define_embassy_dispatcher!(
///   pub fn dispatcher(system: ActorSystem<u32, LocalMailboxRuntime>)
/// );
///
/// fn start(spawner: &Spawner, system: &'static mut ActorSystem<u32, LocalMailboxRuntime>) {
///   spawner.spawn(dispatcher(system)).expect("spawn dispatcher");
/// }
/// ```
#[macro_export]
macro_rules! define_embassy_dispatcher {
  ($vis:vis fn $name:ident(system: $system_ty:ty)) => {
    #[embassy_executor::task]
    $vis async fn $name(system: &'static mut $system_ty) {
      match system.run_forever().await {
        Ok(_) => unreachable!("run_forever must not resolve with Ok"),
        Err(err) => {
          let _ = err;
          #[cfg(debug_assertions)]
          panic!("Embassy dispatcher terminated unexpectedly");
        }
      }
    }
  };
}
