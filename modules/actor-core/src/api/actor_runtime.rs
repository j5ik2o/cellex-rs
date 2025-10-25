//! Actor runtime traits and type aliases.
//!
//! This module defines the high-level `ActorRuntime` trait that wraps a [`MailboxFactory`]
//! and layers actor-system-specific capabilities such as receive timeouts, failure handling,
//! and metrics integration on top of it.

mod base;
mod generic_actor_runtime;

pub use base::*;
pub use generic_actor_runtime::GenericActorRuntime;
