use core::marker::PhantomData;

use crate::api::mailbox::mailbox_options::MailboxOptions;
use crate::api::mailbox::mailbox_producer::MailboxProducer;
use crate::api::mailbox::Mailbox;
use crate::api::mailbox::MailboxPair;
use crate::internal::mailbox::PriorityMailboxBuilder;
use crate::internal::metrics::MetricsSinkShared;
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::Element;

/// Shared handle that can spawn priority mailboxes without exposing the underlying factory.
pub struct PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>, {
  builder: ArcShared<B>,
  metrics_sink: Option<MetricsSinkShared>,
  _marker: PhantomData<M>,
}

impl<M, B> Clone for PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>,
{
  fn clone(&self) -> Self {
    Self {
      builder: self.builder.clone(),
      metrics_sink: self.metrics_sink.clone(),
      _marker: PhantomData,
    }
  }
}

impl<M, B> PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>,
{
  /// Creates a new handle from an `ArcShared`-wrapped factory.
  #[must_use]
  pub fn new(builder: ArcShared<B>) -> Self {
    Self {
      builder,
      metrics_sink: None,
      _marker: PhantomData,
    }
  }

  /// Spawns a priority mailbox using the underlying factory and provided options.
  #[must_use]
  pub fn spawn_mailbox(
    &self,
    options: MailboxOptions,
  ) -> MailboxPair<<B as PriorityMailboxBuilder<M>>::Mailbox, <B as PriorityMailboxBuilder<M>>::Producer> {
    let metrics_sink = self.metrics_sink.clone();
    self.builder.with_ref(|builder| {
      let (mut mailbox, mut producer) = builder.build_priority_mailbox(options);
      if let Some(sink) = metrics_sink.clone() {
        mailbox.set_metrics_sink(Some(sink.clone()));
        producer.set_metrics_sink(Some(sink));
      }
      (mailbox, producer)
    })
  }

  /// Returns the shared builder handle.
  #[must_use]
  pub fn builder(&self) -> ArcShared<B> {
    self.builder.clone()
  }

  /// Configures the metrics sink to be applied to newly spawned mailboxes.
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Mutable setter variant for tests/customization.
  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink;
  }

  /// Returns the configured metrics sink.
  pub fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }
}

impl<M, B> PriorityMailboxSpawnerHandle<M, B>
where
  M: Element,
  B: PriorityMailboxBuilder<M>,
{
  /// Wraps a builder value in `ArcShared` and returns a spawner handle.
  #[must_use]
  pub fn from_builder(builder: B) -> Self {
    Self::new(ArcShared::new(builder))
  }
}

impl<M, R> PriorityMailboxSpawnerHandle<M, R>
where
  M: Element,
  R: PriorityMailboxBuilder<M> + Clone,
{
  /// Wraps a factory implementing [`PriorityMailboxBuilder`] and returns a spawner handle.
  #[must_use]
  pub fn from_factory(factory: R) -> Self {
    Self::from_builder(factory)
  }
}
