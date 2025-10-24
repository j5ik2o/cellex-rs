use cellex_actor_core_rs::api::mailbox::{Mailbox, MailboxOptions};
use cellex_utils_std_rs::{QueueRw, QueueSize, DEFAULT_PRIORITY};

use super::*;

type TestResult<T = ()> = Result<T, String>;

#[cfg(feature = "queue-v2")]
use std::{
  sync::{Arc, Mutex},
  vec::Vec,
};

#[cfg(feature = "queue-v2")]
use cellex_actor_core_rs::api::metrics::{MetricsEvent, MetricsSink, MetricsSinkShared};

#[cfg(feature = "queue-v2")]
#[derive(Clone)]
struct RecordingSink {
  events: Arc<Mutex<Vec<MetricsEvent>>>,
}

#[cfg(feature = "queue-v2")]
impl RecordingSink {
  fn new(events: Arc<Mutex<Vec<MetricsEvent>>>) -> Self {
    Self { events }
  }
}

#[cfg(feature = "queue-v2")]
impl MetricsSink for RecordingSink {
  fn record(&self, event: MetricsEvent) {
    self.events.lock().unwrap().push(event);
  }
}

#[test]
fn priority_mailbox_orders_messages() -> TestResult {
  let factory = TokioPriorityMailboxFactory::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender.send_with_priority(10, DEFAULT_PRIORITY).map_err(|err| format!("send low priority: {:?}", err))?;
  sender
    .send_control_with_priority(99, DEFAULT_PRIORITY + 7)
    .map_err(|err| format!("send high priority: {:?}", err))?;
  sender
    .send_control_with_priority(20, DEFAULT_PRIORITY + 3)
    .map_err(|err| format!("send medium priority: {:?}", err))?;

  let first = mailbox
    .inner()
    .queue()
    .poll()
    .map_err(|err| format!("poll queue first: {:?}", err))?
    .ok_or_else(|| "queue empty for first poll".to_string())?;
  let second = mailbox
    .inner()
    .queue()
    .poll()
    .map_err(|err| format!("poll queue second: {:?}", err))?
    .ok_or_else(|| "queue empty for second poll".to_string())?;
  let third = mailbox
    .inner()
    .queue()
    .poll()
    .map_err(|err| format!("poll queue third: {:?}", err))?
    .ok_or_else(|| "queue empty for third poll".to_string())?;

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 7));
  assert_eq!(second.into_parts(), (20, DEFAULT_PRIORITY + 3));
  assert_eq!(third.into_parts(), (10, DEFAULT_PRIORITY));
  Ok(())
}

#[test]
fn priority_sender_defaults_work() -> TestResult {
  let factory = TokioPriorityMailboxFactory::new(4).with_regular_capacity(4);
  let (mailbox, sender) = factory.mailbox::<u8>(MailboxOptions::default());

  sender.send(PriorityEnvelope::with_default_priority(5)).map_err(|err| format!("send default priority: {:?}", err))?;

  let envelope = mailbox
    .inner()
    .queue()
    .poll()
    .map_err(|err| format!("poll queue: {:?}", err))?
    .ok_or_else(|| "queue empty for default priority poll".to_string())?;
  let (_, priority) = envelope.into_parts();
  assert_eq!(priority, DEFAULT_PRIORITY);
  Ok(())
}

#[test]
fn control_queue_preempts_regular_messages() -> TestResult {
  let factory = TokioPriorityMailboxFactory::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender.send_with_priority(1, DEFAULT_PRIORITY).map_err(|err| format!("enqueue regular message: {:?}", err))?;
  sender
    .send_control_with_priority(99, DEFAULT_PRIORITY + 5)
    .map_err(|err| format!("enqueue control message: {:?}", err))?;

  let first = mailbox
    .inner()
    .queue()
    .poll()
    .map_err(|err| format!("poll queue first: {:?}", err))?
    .ok_or_else(|| "queue empty for control poll".to_string())?;
  let second = mailbox
    .inner()
    .queue()
    .poll()
    .map_err(|err| format!("poll queue second: {:?}", err))?
    .ok_or_else(|| "queue empty for regular poll".to_string())?;

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 5));
  assert_eq!(second.into_parts(), (1, DEFAULT_PRIORITY));
  Ok(())
}

#[test]
fn priority_mailbox_capacity_split() -> TestResult {
  let factory = TokioPriorityMailboxFactory::default();
  let options = MailboxOptions::with_capacities(QueueSize::limited(2), QueueSize::limited(2));
  let (mailbox, sender) = factory.mailbox::<u8>(options);

  assert!(!mailbox.capacity().is_limitless());

  sender.send_control_with_priority(1, DEFAULT_PRIORITY + 2).map_err(|err| format!("control enqueue: {:?}", err))?;
  sender.send_with_priority(2, DEFAULT_PRIORITY).map_err(|err| format!("regular enqueue: {:?}", err))?;
  sender.send_with_priority(3, DEFAULT_PRIORITY).map_err(|err| format!("second regular enqueue: {:?}", err))?;

  let Err(err) = sender.try_send_with_priority(4, DEFAULT_PRIORITY) else {
    return Err("regular capacity not reached".to_string());
  };
  assert!(matches!(&*err, QueueError::Full(_)));
  Ok(())
}

#[cfg(feature = "queue-v2")]
#[test]
fn priority_mailbox_emits_growth_metric() -> TestResult {
  let factory = TokioPriorityMailboxFactory::new(4).with_regular_capacity(0);
  let (mut mailbox, mut sender) = factory.mailbox::<u32>(MailboxOptions::default());

  let events = Arc::new(Mutex::new(Vec::new()));
  let sink = MetricsSinkShared::new(RecordingSink::new(events.clone()));

  mailbox.set_metrics_sink(Some(sink.clone()));
  sender.set_metrics_sink(Some(sink.clone()));

  sender
    .send(PriorityEnvelope::with_default_priority(1u32))
    .map_err(|err| format!("regular enqueue should succeed: {err:?}"))?;

  let recorded = events.lock().unwrap().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxGrewTo { capacity } if *capacity >= 1)),
    "expected MailboxGrewTo event, recorded: {recorded:?}"
  );

  Ok(())
}
