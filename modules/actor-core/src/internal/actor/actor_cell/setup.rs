use super::*;

impl<M, R, Strat> ActorCell<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    actor_id: ActorId,
    map_system: MapSystemShared<M>,
    watchers: Vec<ActorId>,
    actor_path: ActorPath,
    mailbox_runtime: R,
    mailbox_spawner: PriorityMailboxSpawnerHandle<M, R>,
    mailbox: R::Mailbox<PriorityEnvelope<M>>,
    sender: R::Producer<PriorityEnvelope<M>>,
    supervisor: Box<dyn Supervisor<M>>,
    handler: Box<ActorHandlerFn<M, R>>,
    receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
    extensions: Extensions,
  ) -> Self {
    let mut cell = Self {
      actor_id,
      map_system,
      watchers,
      actor_path,
      mailbox_runtime,
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      _strategy: PhantomData,
      stopped: false,
      receive_timeout_factory: None,
      receive_timeout_scheduler: None,
      extensions,
    };
    cell.configure_receive_timeout_factory(receive_timeout_factory);
    cell
  }

  pub(crate) fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>)
  where
    R: MailboxRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<M>>: Clone,
    R::Signal: Clone,
    R::Producer<PriorityEnvelope<M>>: Clone, {
    Mailbox::set_metrics_sink(&mut self.mailbox, sink.clone());
    MailboxProducer::set_metrics_sink(&mut self.sender, sink.clone());
    self.mailbox_spawner.set_metrics_sink(sink);
  }

  pub(crate) fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>)
  where
    R: MailboxRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<M>>: Clone,
    R::Signal: Clone,
    R::Producer<PriorityEnvelope<M>>: Clone, {
    Mailbox::set_scheduler_hook(&mut self.mailbox, hook.clone());
    MailboxProducer::set_scheduler_hook(&mut self.sender, hook);
  }
}
