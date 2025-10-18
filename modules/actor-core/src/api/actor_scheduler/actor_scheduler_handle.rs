use alloc::boxed::Box;

use crate::api::actor_scheduler::ActorScheduler;

/// Type alias for boxed scheduler instances returned by builders.
pub type ActorSchedulerHandle<M, MF> = Box<dyn ActorScheduler<M, MF>>;
