use core::future::Future;

use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError, QueueRw, QueueSize};

use crate::runtime::message::{DynMessage, MetadataStorageMode};
use crate::runtime::metrics::MetricsSinkShared;
use crate::runtime::scheduler::{ReadyQueueHandle, SchedulerBuilder};
use crate::{
  FailureEventHandler, FailureEventListener, PriorityEnvelope, PriorityMailboxSpawnerHandle,
  ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared,
};

use super::queue_mailbox::{MailboxOptions, QueueMailbox, QueueMailboxProducer};

/// Type alias for mailbox and producer pair.
///
/// Pair of receiver and sender handles returned when creating a mailbox.
pub type MailboxPair<H, P> = (H, P);

/// Mailbox abstraction that decouples message queue implementations from core logic.
///
/// Abstraction trait that decouples message queue implementations from core logic.
/// Enables unified handling of various queue implementations (bounded/unbounded, prioritized, etc.).
///
/// # Type Parameters
/// - `M`: Type of the message to process
pub trait Mailbox<M> {
  /// Error type for message sending
  type SendError;

  /// Future type for message reception
  type RecvFuture<'a>: Future<Output = Result<M, QueueError<M>>> + 'a
  where
    Self: 'a;

  /// Attempts to send a message (non-blocking).
  ///
  /// # Arguments
  /// - `message`: Message to send
  ///
  /// # Returns
  /// `Ok(())` on success, `Err(SendError)` on failure
  fn try_send(&self, message: M) -> Result<(), Self::SendError>;

  /// Receives a message asynchronously.
  ///
  /// # Returns
  /// Future for message reception
  fn recv(&self) -> Self::RecvFuture<'_>;

  /// Gets the number of messages in the mailbox.
  ///
  /// Default implementation returns unlimited.
  fn len(&self) -> QueueSize {
    QueueSize::limitless()
  }

  /// Gets the capacity of the mailbox.
  ///
  /// Default implementation returns unlimited.
  fn capacity(&self) -> QueueSize {
    QueueSize::limitless()
  }

  /// Checks if the mailbox is empty.
  ///
  /// # Returns
  /// `true` if empty, `false` if there are messages
  fn is_empty(&self) -> bool {
    self.len() == QueueSize::Limited(0)
  }

  /// Closes the mailbox.
  ///
  /// Default implementation does nothing.
  fn close(&self) {}

  /// Checks if the mailbox is closed.
  ///
  /// Default implementation always returns `false`.
  ///
  /// # Returns
  /// `true` if closed, `false` if open
  fn is_closed(&self) -> bool {
    false
  }

  /// Injects a metrics sink for enqueue instrumentation. Default: no-op.
  fn set_metrics_sink(&mut self, _sink: Option<MetricsSinkShared>) {}

  /// Installs a scheduler hook invoked on message arrivals. Default: no-op.
  fn set_scheduler_hook(&mut self, _hook: Option<ReadyQueueHandle>) {}
}

/// Shared interface exposed by mailbox handles that can be managed by the runtime scheduler.
pub trait MailboxHandle<M>: Mailbox<M> + Clone
where
  M: Element, {
  /// Associated signal type used to block until new messages arrive.
  type Signal: MailboxSignal;

  /// Clones the underlying signal for waiters.
  fn signal(&self) -> Self::Signal;

  /// Attempts to dequeue one message without waiting.
  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>>;
}

/// Sending interface exposed by mailbox producers that enqueue messages.
pub trait MailboxProducer<M>: Clone
where
  M: Element, {
  /// Attempts to enqueue a message without waiting.
  fn try_send(&self, message: M) -> Result<(), QueueError<M>>;

  /// Injects a metrics sink for enqueue instrumentation. Default: no-op.
  fn set_metrics_sink(&mut self, _sink: Option<MetricsSinkShared>) {}

  /// Installs a scheduler hook invoked on message arrivals. Default: no-op.
  fn set_scheduler_hook(&mut self, _hook: Option<ReadyQueueHandle>) {}
}

/// Notification primitive used by `QueueMailbox` to park awaiting receivers until
/// new messages are available.
///
/// Synchronization primitive used for notifying message arrivals.
/// Provides a mechanism for receivers to wait for messages and senders to notify arrivals.
pub trait MailboxSignal: Clone {
  /// Future type for waiting
  type WaitFuture<'a>: Future<Output = ()> + 'a
  where
    Self: 'a;

  /// Notifies waiting receivers that a message has arrived.
  fn notify(&self);

  /// Waits for a message arrival.
  ///
  /// # Returns
  /// Future that waits for notification
  fn wait(&self) -> Self::WaitFuture<'_>;
}

/// Marker trait describing the synchronization requirements for a mailbox factory.
pub trait MailboxConcurrency: Copy + 'static {}

/// Thread-safe mailbox mode requiring `Send + Sync` types.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadSafe;

impl MailboxConcurrency for ThreadSafe {}

/// Single-threaded mailbox mode without additional synchronization requirements.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleThread;

impl MailboxConcurrency for SingleThread {}

/// Factory trait for creating mailboxes.
///
/// Generates mailbox and queue implementations according to
/// specific async runtimes (Tokio, Async-std, etc.).
pub trait MailboxRuntime {
  /// Declares the concurrency mode for this factory.
  type Concurrency: MailboxConcurrency + MetadataStorageMode;

  /// Type of notification signal
  type Signal: MailboxSignal;

  /// Type of message queue
  type Queue<M>: QueueRw<M> + Clone
  where
    M: Element;

  /// Mailbox handle returned to the scheduler.
  type Mailbox<M>: MailboxHandle<M, Signal = Self::Signal> + Clone
  where
    M: Element;

  /// Producer handle used for enqueuing messages into the mailbox.
  type Producer<M>: MailboxProducer<M> + Clone
  where
    M: Element;

  /// Creates a mailbox with the specified options.
  ///
  /// # Arguments
  /// - `options`: Capacity settings for the mailbox
  ///
  /// # Returns
  /// Pair of `(mailbox, producer)`
  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element;

  /// Creates a mailbox with default settings.
  ///
  /// # Returns
  /// Pair of `(mailbox, producer)`
  fn build_default_mailbox<M>(&self) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    self.build_mailbox(MailboxOptions::default())
  }
}

/// High-level runtime interface that extends [`MailboxRuntime`] with bundle-specific capabilities.
#[allow(dead_code)]
pub trait ActorRuntime: Clone {
  /// Underlying mailbox runtime retained by this actor runtime facade.
  type Mailbox: MailboxRuntime + Clone;

  /// Returns a shared reference to the underlying mailbox runtime.
  fn mailbox_runtime(&self) -> &Self::Mailbox;

  /// Consumes `self` and returns the underlying mailbox runtime.
  fn into_mailbox_runtime(self) -> Self::Mailbox
  where
    Self: Sized;

  /// Returns the shared handle to the underlying mailbox runtime.
  fn mailbox_runtime_shared(&self) -> ArcShared<Self::Mailbox>;

  /// Returns the receive-timeout factory configured for this runtime.
  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self>>;

  /// Returns the receive-timeout driver configured for this runtime.
  fn receive_timeout_driver(&self) -> Option<ReceiveTimeoutDriverShared<Self::Mailbox>>;

  /// Overrides the receive-timeout factory using the base mailbox runtime type.
  fn with_receive_timeout_factory(self, factory: ReceiveTimeoutFactoryShared<DynMessage, Self::Mailbox>) -> Self
  where
    Self: Sized;

  /// Overrides the receive-timeout factory using a runtime-specific factory.
  fn with_receive_timeout_factory_shared(self, factory: ReceiveTimeoutFactoryShared<DynMessage, Self>) -> Self
  where
    Self: Sized;

  /// Overrides the receive-timeout driver.
  fn with_receive_timeout_driver(self, driver: Option<ReceiveTimeoutDriverShared<Self::Mailbox>>) -> Self
  where
    Self: Sized;

  /// Mutably overrides the receive-timeout driver.
  fn set_receive_timeout_driver(&mut self, driver: Option<ReceiveTimeoutDriverShared<Self::Mailbox>>);

  /// Returns a factory constructed from the configured receive-timeout driver, if any.
  fn receive_timeout_driver_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self>>;

  /// Returns the root failure event listener configured for the runtime.
  fn root_event_listener(&self) -> Option<FailureEventListener>;

  /// Overrides the root failure event listener.
  fn with_root_event_listener(self, listener: Option<FailureEventListener>) -> Self
  where
    Self: Sized;

  /// Returns the root escalation handler configured for the runtime.
  fn root_escalation_handler(&self) -> Option<FailureEventHandler>;

  /// Overrides the root escalation handler.
  fn with_root_escalation_handler(self, handler: Option<FailureEventHandler>) -> Self
  where
    Self: Sized;

  /// Returns the metrics sink configured for the runtime.
  fn metrics_sink(&self) -> Option<MetricsSinkShared>;

  /// Overrides the metrics sink.
  fn with_metrics_sink(self, sink: Option<MetricsSinkShared>) -> Self
  where
    Self: Sized;

  /// Overrides the metrics sink using a shared handle.
  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self
  where
    Self: Sized;

  /// Returns a priority mailbox spawner handle without exposing the internal factory.
  fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, Self::Mailbox>
  where
    M: Element,
    <<Self as ActorRuntime>::Mailbox as crate::MailboxRuntime>::Queue<PriorityEnvelope<M>>: Clone,
    <<Self as ActorRuntime>::Mailbox as crate::MailboxRuntime>::Signal: Clone;

  /// Overrides the scheduler builder used during actor system construction.
  fn with_scheduler_builder(self, builder: SchedulerBuilder<DynMessage, Self>) -> Self
  where
    Self: Sized + MailboxRuntime;

  /// Overrides the scheduler builder using a shared handle.
  fn with_scheduler_builder_shared(self, builder: ArcShared<SchedulerBuilder<DynMessage, Self>>) -> Self
  where
    Self: Sized + MailboxRuntime;

  /// Returns the scheduler builder configured for this runtime.
  fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, Self>>
  where
    Self: MailboxRuntime;
}

impl<M, Q, S> MailboxHandle<M> for QueueMailbox<Q, S>
where
  Q: QueueRw<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  type Signal = S;

  fn signal(&self) -> Self::Signal {
    self.signal().clone()
  }

  fn try_dequeue(&self) -> Result<Option<M>, QueueError<M>> {
    self.queue().poll()
  }
}

impl<M, Q, S> MailboxProducer<M> for QueueMailboxProducer<Q, S>
where
  Q: QueueRw<M> + Clone,
  S: MailboxSignal,
  M: Element,
{
  fn try_send(&self, message: M) -> Result<(), QueueError<M>> {
    <QueueMailboxProducer<Q, S>>::try_send(self, message)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    <QueueMailboxProducer<Q, S>>::set_metrics_sink(self, sink);
  }

  fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {
    <QueueMailboxProducer<Q, S>>::set_scheduler_hook(self, hook);
  }
}
