//! Actor API aggregation module.
//!
//! Re-exports basic types used by the runtime layer such as
//! [`SystemMessage`] and [`PriorityEnvelope`] from this module.

mod actor_ref;
mod ask;
mod behavior;
mod context;
mod failure;
mod props;
mod root_context;
mod shutdown_token;
#[cfg(test)]
mod tests;

pub use crate::api::actor_runtime::GenericActorRuntime;
pub use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxOf, MailboxQueueOf, MailboxSignalOf};
pub use crate::api::actor_system::{
  ActorSystem, ActorSystemBuilder, ActorSystemConfig, ActorSystemRunner, Spawn, Timer,
};
pub use crate::api::mailbox::MailboxRuntime;
pub use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
pub use crate::DynMessage as RuntimeMessage;
pub use actor_ref::ActorRef;
pub use actor_ref::PriorityActorRef;
pub use ask::{ask_with_timeout, AskError, AskFuture, AskResult, AskTimeoutFuture};
pub use behavior::{ActorAdapter, Behavior, BehaviorDirective, Behaviors, SupervisorStrategy};
pub use context::{Context, ContextLogLevel, ContextLogger, MessageAdapterRef, SetupContext};
pub use failure::{ActorFailure, BehaviorFailure, DefaultBehaviorFailure};
pub use props::Props;
pub use root_context::RootContext;
pub use shutdown_token::ShutdownToken;

#[doc(hidden)]
mod __actor_doc_refs {
  use super::*;
  use crate::DynMessage;
  use cellex_utils_core_rs::Element;

  #[allow(dead_code)]
  pub fn _priority_envelope_marker<M: Element>() {
    let _ = core::mem::size_of::<PriorityEnvelope<DynMessage>>();
    let _ = core::mem::size_of::<PriorityEnvelope<M>>();
  }

  #[allow(dead_code)]
  pub fn _system_message_marker(message: SystemMessage) -> SystemMessage {
    message
  }

  #[allow(dead_code)]
  pub fn _mailbox_runtime_marker<R: MailboxRuntime>(mailbox_runtime: &R) -> &R {
    mailbox_runtime
  }
}
