use super::*;

impl<M, R, Strat> ActorCell<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  pub(in crate::internal) fn configure_receive_timeout_factory(
    &mut self,
    factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
  ) {
    if let Some(cell) = self.receive_timeout_scheduler.as_ref() {
      cell.borrow_mut().cancel();
    }
    self.receive_timeout_scheduler = None;
    self.receive_timeout_factory = factory.clone();
    if let Some(factory_arc) = factory {
      let scheduler = factory_arc.create(self.sender.clone(), self.map_system.clone());
      self.receive_timeout_scheduler = Some(RefCell::new(scheduler));
    }
  }

  pub(super) fn mark_stopped(&mut self, guardian: &mut Guardian<M, R, Strat>) {
    if self.stopped {
      return;
    }

    self.stopped = true;
    if let Some(cell) = self.receive_timeout_scheduler.as_ref() {
      cell.borrow_mut().cancel();
    }
    self.receive_timeout_scheduler = None;
    self.receive_timeout_factory = None;
    self.mailbox.close();
    let _ = guardian.remove_child(self.actor_id);
    self.watchers.clear();
  }

  pub(super) fn should_mark_stop_for_message() -> bool {
    TypeId::of::<M>() == TypeId::of::<DynMessage>()
  }
}
