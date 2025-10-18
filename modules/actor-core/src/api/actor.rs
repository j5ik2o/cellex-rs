//! Actor API aggregation module.

/// Actor execution context
pub mod actor_context;
/// Actor failure information
pub mod actor_failure;
mod actor_id;
mod actor_path;
/// Actor reference types
pub mod actor_ref;
/// Ask pattern for request-response communication
pub mod ask;
/// Actor behavior definitions
pub mod behavior;
mod child_naming;
mod message_adapter_ref;
mod message_metadata_responder;
/// Actor spawn properties
mod props;
/// Root context for top-level actors
pub mod root_context;
/// Shutdown coordination
pub mod shutdown_token;
/// Actor lifecycle signals
pub mod signal;
mod spawn;
mod spawn_error;
#[cfg(test)]
mod tests;
mod timer;

pub use actor_id::ActorId;
pub use actor_path::ActorPath;
pub use child_naming::ChildNaming;
pub use message_adapter_ref::MessageAdapterRef;
pub use message_metadata_responder::MessageMetadataResponder;
pub use props::Props;
pub use spawn::Spawn;
pub use spawn_error::SpawnError;
pub use timer::Timer;

use crate::{api::actor::actor_failure::ActorFailure, internal::actor_context::InternalActorContext};

/// Type alias representing the dynamically-dispatched actor handler invoked by schedulers.
pub type TypedActorHandlerFn<U, AR> = dyn for<'r, 'ctx> FnMut(
    &mut crate::api::actor::actor_context::ActorContext<'r, 'ctx, U, AR>,
    U,
  ) -> Result<(), ActorFailure>
  + 'static;

pub(crate) type ActorHandlerFn<M, MF> =
  dyn for<'ctx> FnMut(&mut InternalActorContext<'ctx, MF>, M) -> Result<(), ActorFailure> + 'static;
