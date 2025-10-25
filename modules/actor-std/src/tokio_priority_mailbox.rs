#[cfg(test)]
mod tests;

mod factory;
mod mailbox;
#[cfg(feature = "queue-v2")]
mod priority_sync_driver;
mod queues;
mod sender;

use cellex_actor_core_rs::shared::mailbox::messages::PriorityEnvelope;
use cellex_utils_std_rs::QueueError;

type PriorityQueueError<M> = Box<QueueError<PriorityEnvelope<M>>>;

pub use factory::TokioPriorityMailboxFactory;
pub use mailbox::TokioPriorityMailbox;
pub use sender::TokioPriorityMailboxSender;

use crate::tokio_mailbox::NotifySignal;
