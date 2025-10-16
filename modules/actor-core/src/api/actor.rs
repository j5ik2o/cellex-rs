//! Actor API aggregation module.
//!
//! Re-exports basic types used by the runtime layer such as
//! [`SystemMessage`] and [`PriorityEnvelope`] from this module.

mod actor_ref;
mod actor_system;
mod actor_system_builder;
mod actor_system_config;
mod actor_system_runner;
mod actor_system_support;
mod ask;
mod behavior;
mod context;
mod failure;
mod generic_runtime;
mod props;
mod root_context;
mod shutdown_token;
#[cfg(test)]
mod tests;

pub use crate::internal::mailbox::{
  Mailbox, MailboxOptions, MailboxPair, MailboxRuntime, MailboxSignal, PriorityEnvelope, QueueMailbox,
  QueueMailboxProducer, QueueMailboxRecv, SystemMessage,
};
pub use crate::internal::message::DynMessage as RuntimeMessage;
pub use actor_ref::ActorRef;
pub use actor_system::ActorSystem;
pub use actor_system_builder::ActorSystemBuilder;
pub use actor_system_config::ActorSystemConfig;
pub use actor_system_runner::ActorSystemRunner;
pub use actor_system_support::{Spawn, Timer};
pub use ask::{ask_with_timeout, AskError, AskFuture, AskResult, AskTimeoutFuture};
pub use behavior::{ActorAdapter, Behavior, BehaviorDirective, Behaviors, SupervisorStrategy};
pub use context::{Context, ContextLogLevel, ContextLogger, MessageAdapterRef, SetupContext};
pub use failure::{ActorFailure, BehaviorFailure, DefaultBehaviorFailure};
pub use generic_runtime::GenericActorRuntime;
pub use props::Props;
pub use root_context::RootContext;
pub use shutdown_token::ShutdownToken;

#[doc(hidden)]
mod __actor_doc_refs {
  use super::*;
  use crate::internal::message::DynMessage;
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
