/// Internal actor implementation
pub(crate) mod actor;
pub(crate) mod actor_context;
pub(crate) mod actor_system;
pub(crate) mod mailbox;
/// Internal message metadata storage and dispatch primitives.
pub mod message;
mod runtime_state;
pub(crate) mod supervision;

pub(crate) use runtime_state::GenericActorRuntimeState;
