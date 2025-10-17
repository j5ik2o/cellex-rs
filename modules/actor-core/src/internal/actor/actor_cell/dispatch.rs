use super::*;

impl<M, R, Strat> ActorCell<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  pub(super) fn dispatch_envelope(
    &mut self,
    envelope: PriorityEnvelope<M>,
    guardian: &mut Guardian<M, R, Strat>,
    new_children: &mut Vec<ActorCell<M, R, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    if self.stopped {
      return Ok(());
    }

    let should_stop =
      matches!(envelope.system_message(), Some(SystemMessage::Stop)) && Self::should_mark_stop_for_message();
    if let Some(SystemMessage::Escalate(failure)) = envelope.system_message().cloned() {
      if let Some(next_failure) = guardian.escalate_failure(failure)? {
        escalations.push(next_failure);
      }
      return Ok(());
    }

    let influences_receive_timeout = envelope.system_message().is_none();
    let (message, priority) = envelope.into_parts();
    self.supervisor.before_handle();
    let mut pending_specs = Vec::new();

    #[cfg(feature = "unwind-supervision")]
    {
      let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        self.invoke_handler(message, priority, influences_receive_timeout, &mut pending_specs)
      }));

      return match result {
        Ok(handler_result) => self.apply_handler_result(
          handler_result,
          pending_specs,
          should_stop,
          guardian,
          new_children,
          escalations,
        ),
        Err(payload) => {
          let failure = ActorFailure::from_panic_payload(payload.as_ref());
          if let Some(info) = guardian.notify_failure(self.actor_id, failure)? {
            escalations.push(info);
          }
          Ok(())
        }
      };
    }

    #[cfg(not(feature = "unwind-supervision"))]
    {
      let handler_result = self.invoke_handler(message, priority, influences_receive_timeout, &mut pending_specs);

      self.apply_handler_result(
        handler_result,
        pending_specs,
        should_stop,
        guardian,
        new_children,
        escalations,
      )
    }
  }

  fn invoke_handler(
    &mut self,
    message: M,
    priority: i8,
    influences_receive_timeout: bool,
    pending_specs: &mut Vec<ChildSpawnSpec<M, R>>,
  ) -> Result<(), ActorFailure> {
    let receive_timeout = self.receive_timeout_scheduler.as_ref();
    let mut ctx = ActorContext::new(
      &self.mailbox_runtime,
      self.mailbox_spawner.clone(),
      &self.sender,
      self.supervisor.as_mut(),
      pending_specs,
      self.map_system.clone(),
      self.actor_path.clone(),
      self.actor_id,
      &mut self.watchers,
      receive_timeout,
      self.extensions.clone(),
    );
    ctx.enter_priority(priority);
    let handler_result = (self.handler)(&mut ctx, message);
    ctx.notify_receive_timeout_activity(influences_receive_timeout);
    ctx.exit_priority();
    self.supervisor.after_handle();
    handler_result
  }

  #[allow(clippy::too_many_arguments)]
  fn apply_handler_result(
    &mut self,
    handler_result: Result<(), ActorFailure>,
    pending_specs: Vec<ChildSpawnSpec<M, R>>,
    should_stop: bool,
    guardian: &mut Guardian<M, R, Strat>,
    new_children: &mut Vec<ActorCell<M, R, Strat>>,
    escalations: &mut Vec<FailureInfo>,
  ) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    match handler_result {
      Ok(()) => {
        for spec in pending_specs.into_iter() {
          self
            .register_child_from_spec(spec, guardian, new_children)
            .map_err(|err| match err {
              SpawnError::Queue(queue_err) => queue_err,
              SpawnError::NameExists(name) => panic!("unexpected named spawn conflict: {name}"),
            })?;
        }
        if should_stop {
          self.mark_stopped(guardian);
        }
        Ok(())
      }
      Err(err) => {
        if let Some(info) = guardian.notify_failure(self.actor_id, err)? {
          escalations.push(info);
        }
        Ok(())
      }
    }
  }

  fn register_child_from_spec(
    &mut self,
    spec: ChildSpawnSpec<M, R>,
    guardian: &mut Guardian<M, R, Strat>,
    new_children: &mut Vec<ActorCell<M, R, Strat>>,
  ) -> Result<(), SpawnError<M>> {
    let ChildSpawnSpec {
      mailbox,
      sender,
      supervisor,
      handler,
      mailbox_spawner,
      watchers,
      map_system,
      parent_path,
      extensions,
      child_naming,
    } = spec;

    let control_ref = PriorityActorRef::new(sender.clone());
    let primary_watcher = watchers.first().copied();
    let (actor_id, actor_path) = guardian.register_child_with_naming(
      control_ref,
      map_system.clone(),
      primary_watcher,
      &parent_path,
      child_naming,
    )?;
    let mut cell = ActorCell::new(
      actor_id,
      map_system,
      watchers,
      actor_path,
      self.mailbox_runtime.clone(),
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      self.receive_timeout_factory.clone(),
      extensions,
    );
    let sink = self.mailbox_spawner.metrics_sink();
    cell.set_metrics_sink(sink);
    new_children.push(cell);
    Ok(())
  }
}
