#[cfg(test)]
mod tests;

mod factory;
mod mailbox;
mod priority_sync_driver;
mod sender;

use cellex_actor_core_rs::shared::mailbox::messages::PriorityEnvelope;
use cellex_utils_core_rs::collections::queue::backend::QueueError;

type PriorityQueueError<M> = Box<QueueError<PriorityEnvelope<M>>>;

pub use factory::TokioPriorityMailboxFactory;
pub use mailbox::TokioPriorityMailbox;
pub use sender::TokioPriorityMailboxSender;

use crate::tokio_mailbox::NotifySignal;
