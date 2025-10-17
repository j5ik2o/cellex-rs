/// Internal actor implementation
pub(crate) mod actor;
pub(crate) mod actor_system;
pub(crate) mod context;
/// Guardian supervision tree utilities used for internal actor bootstrapping.
pub mod guardian;
pub(crate) mod mailbox;
/// Internal message metadata storage and dispatch primitives.
pub mod message;
pub(crate) mod runtime_state;
/// Internal schedulers coordinating actor execution and supervision.
pub mod scheduler;
pub(crate) mod supervision;
