#![cfg(feature = "alloc")]

//! New runtime API surface built on top of shared runtime components.

pub mod actor_system;
pub mod bundle;
pub mod mailbox;
pub mod runtime_parts;
pub mod scheduler;
#[cfg(any(test, feature = "test-support"))]
pub mod test_harness;

pub use actor_system::{NewActorSystem, NewInternalActorSystem};
pub use bundle::NewActorRuntimeBundle;
pub use mailbox::{NewMailboxHandleFactory, NewMailboxRuntime};
pub use runtime_parts::RuntimeParts;
pub use scheduler::{NewSchedulerBuilder, SharedSchedulerBuilder};
#[cfg(any(test, feature = "test-support"))]
pub use test_harness::TestHarnessBundle;
