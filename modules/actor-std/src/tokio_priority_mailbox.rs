#[cfg(test)]
mod tests;

mod mailbox;
mod queues;
mod runtime;
mod sender;

use cellex_actor_core_rs::shared::mailbox::messages::PriorityEnvelope;
use cellex_utils_std_rs::QueueError;

type PriorityQueueError<M> = Box<QueueError<PriorityEnvelope<M>>>;

pub use mailbox::TokioPriorityMailbox;
pub use runtime::TokioPriorityMailboxRuntime;
pub use sender::TokioPriorityMailboxSender;

use crate::tokio_mailbox::NotifySignal;
