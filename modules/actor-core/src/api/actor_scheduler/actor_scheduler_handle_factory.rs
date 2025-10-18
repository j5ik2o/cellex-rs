use crate::api::{actor_scheduler::actor_scheduler_handle::ActorSchedulerHandle, extensions::Extensions};

/// Type alias for factory closures that produce [`ActorSchedulerHandle`] values.
#[cfg(target_has_atomic = "ptr")]
pub type ActorSchedulerHandleFactoryFn<M, MF> =
  dyn Fn(MF, Extensions) -> ActorSchedulerHandle<M, MF> + Send + Sync + 'static;
/// Type alias for factory closures that produce [`ActorSchedulerHandle`] values on single-threaded
/// targets.
#[cfg(not(target_has_atomic = "ptr"))]
pub type ActorSchedulerHandleFactoryFn<M, MF> = dyn Fn(MF, Extensions) -> ActorSchedulerHandle<M, MF> + 'static;
