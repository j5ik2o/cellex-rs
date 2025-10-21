//! Actor runtime traits and type aliases.
//!
//! This module defines the high-level `ActorRuntime` trait that extends
//! `MailboxRuntime` with actor-system-specific capabilities such as
//! receive timeouts, failure handling, and metrics integration.

mod base;
mod generic_actor_runtime;
#[cfg(test)]
mod tests;

pub use base::*;
pub use generic_actor_runtime::GenericActorRuntime;
