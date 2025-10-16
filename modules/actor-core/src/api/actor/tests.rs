#![cfg(feature = "std")]
#![allow(deprecated)]

use super::actor_runtime::{ActorRuntime, MailboxQueueOf, MailboxSignalOf};
use super::ask::create_ask_handles;
use super::behavior::{Signal, SupervisorStrategyConfig};
use super::*;
use super::{ask_with_timeout, AskError};
use crate::api::guardian::AlwaysRestart;
use crate::api::{InternalMessageSender, MessageEnvelope, MessageMetadata, MessageSender};
use crate::internal::mailbox::test_support::TestMailboxRuntime;
use crate::internal::mailbox::traits::MailboxRuntime;
use crate::internal::message::{take_metadata, DynMessage};
use crate::internal::scheduler::SpawnError;
use crate::next_extension_id;
use crate::ActorId;
use crate::MapSystemShared;
use crate::PriorityEnvelope;
use crate::SystemMessage;
use crate::ThreadSafe;
use crate::{serializer_extension_id, SerializerRegistryExtension};
use crate::{Extension, ExtensionId};
use crate::{FailureEvent, FailureEventListener};
use alloc::rc::Rc;
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
use alloc::string::String;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::vec::Vec;
use cellex_serialization_json_rs::SERDE_JSON_SERIALIZER_ID;
use cellex_utils_core_rs::{Element, QueueError};
use core::cell::RefCell;
use core::future::Future;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};
use serde::{Deserialize, Serialize};
use serde_json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc as StdArc, Mutex};

type TestRuntime = GenericActorRuntime<TestMailboxRuntime>;

#[derive(Clone, Debug)]
struct ParentMessage(String);

#[derive(Clone, Debug)]
struct ChildMessage {
  text: String,
}

mod ready_queue_worker_configuration {
  #[test]
  fn actor_system_spawn_prefix_allows_multiple_spawns() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime);
    let mut system: ActorSystem<u32, _> =
      ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());
    let mut root = system.root_context();

    root
      .spawn_prefix(Props::new(|_, _: u32| Ok(())), "worker")
      .expect("spawn worker-0");
    root
      .spawn_prefix(Props::new(|_, _: u32| Ok(())), "worker")
      .expect("spawn worker-1");
  }

  #[test]
  fn actor_system_spawn_named_rejects_duplicate_names() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime);
    let mut system: ActorSystem<u32, _> =
      ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());
    let mut root = system.root_context();

    root
      .spawn_named(Props::new(|_, _: u32| Ok(())), "service")
      .expect("spawn service");
    match root.spawn_named(Props::new(|_, _: u32| Ok(())), "service") {
      Err(SpawnError::NameExists(name)) => assert_eq!(name, "service"),
      Err(SpawnError::Queue(err)) => panic!("unexpected queue error: {:?}", err),
      Ok(_) => panic!("expected duplicate name error"),
    }
  }

  use super::*;
  use crate::internal::mailbox::test_support::TestMailboxRuntime;

  type TestRuntime = GenericActorRuntime<TestMailboxRuntime>;

  #[test]
  fn actor_system_runner_defaults_ready_queue_worker_count_to_one() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime.clone());
    let system: ActorSystem<u32, _> = ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());
    let runner = system.into_runner();

    assert_eq!(runner.ready_queue_worker_count().get(), 1);
  }

  #[test]
  fn actor_system_runner_respects_configured_ready_queue_worker_count() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime.clone());
    let worker_count = NonZeroUsize::new(3).expect("non-zero worker count");
    let config = ActorSystemConfig::default().with_ready_queue_worker_count(Some(worker_count));

    let system: ActorSystem<u32, _> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
    let runner = system.into_runner();

    assert_eq!(runner.ready_queue_worker_count(), worker_count);
  }

  #[test]
  fn actor_system_runner_allows_overriding_worker_count() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime);
    let worker_count = NonZeroUsize::new(4).expect("non-zero worker count");
    let config = ActorSystemConfig::default().with_ready_queue_worker_count(Some(worker_count));

    let system: ActorSystem<u32, _> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
    let updated = NonZeroUsize::new(6).expect("non-zero worker count");
    let runner = system.into_runner().with_ready_queue_worker_count(updated);

    assert_eq!(runner.ready_queue_worker_count(), updated);
  }
}

mod builder_api {
  use super::*;
  use crate::internal::mailbox::test_support::TestMailboxRuntime;

  #[test]
  fn actor_system_builder_applies_ready_queue_override() {
    let actor_runtime: TestRuntime = GenericActorRuntime::new(TestMailboxRuntime::unbounded());
    let worker_count = NonZeroUsize::new(3).expect("non-zero worker count");

    let system: ActorSystem<u32, _> = ActorSystem::builder(actor_runtime)
      .configure(|config| config.set_ready_queue_worker_count(Some(worker_count)))
      .build();
    let runner = system.into_runner();

    assert_eq!(runner.ready_queue_worker_count(), worker_count);
  }
}

mod receive_timeout_injection {
  use super::TestRuntime;
  use super::*;
  use crate::internal::mailbox::test_support::TestMailboxRuntime;
  use crate::internal::scheduler::receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory};
  use crate::{
    ActorRuntime, ActorSystem, ActorSystemConfig, DynMessage, GenericActorRuntime, MapSystemShared, PriorityEnvelope,
    ReceiveTimeoutDriver, ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared,
  };
  use alloc::boxed::Box;
  use core::time::Duration;
  use futures::executor::block_on;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Arc;

  #[derive(Clone)]
  struct CountingFactory {
    calls: Arc<AtomicUsize>,
  }

  impl CountingFactory {
    fn new(calls: Arc<AtomicUsize>) -> Self {
      Self { calls }
    }
  }

  struct CountingScheduler;

  impl ReceiveTimeoutScheduler for CountingScheduler {
    fn set(&mut self, _duration: Duration) {}

    fn cancel(&mut self) {}

    fn notify_activity(&mut self) {}
  }

  impl ReceiveTimeoutSchedulerFactory<DynMessage, TestMailboxRuntime> for CountingFactory {
    fn create(
      &self,
      _sender: <TestMailboxRuntime as MailboxRuntime>::Producer<PriorityEnvelope<DynMessage>>,
      _map_system: MapSystemShared<DynMessage>,
    ) -> Box<dyn ReceiveTimeoutScheduler> {
      self.calls.fetch_add(1, Ordering::SeqCst);
      Box::new(CountingScheduler)
    }
  }

  #[derive(Clone)]
  struct CountingDriver {
    driver_calls: Arc<AtomicUsize>,
    factory_calls: Arc<AtomicUsize>,
  }

  impl CountingDriver {
    fn new(driver_calls: Arc<AtomicUsize>, factory_calls: Arc<AtomicUsize>) -> Self {
      Self {
        driver_calls,
        factory_calls,
      }
    }
  }

  impl ReceiveTimeoutDriver<TestMailboxRuntime> for CountingDriver {
    fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, TestMailboxRuntime> {
      self.driver_calls.fetch_add(1, Ordering::SeqCst);
      ReceiveTimeoutFactoryShared::new(CountingFactory::new(self.factory_calls.clone()))
    }
  }

  fn spawn_test_actor<R: ActorRuntime>(system: &mut ActorSystem<u32, R, AlwaysRestart>) {
    let props = Props::new(|_, _: u32| Ok(()));
    let mut root = system.root_context();
    let actor_ref = root.spawn(props).expect("spawn actor");
    actor_ref.tell(0).expect("tell");
    block_on(root.dispatch_next()).expect("dispatch");
  }

  #[test]
  fn actor_system_uses_driver_receive_timeout_when_no_bundle_or_config() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let driver_calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = Arc::new(AtomicUsize::new(0));

    let actor_runtime: TestRuntime =
      GenericActorRuntime::new(mailbox_runtime.clone()).with_receive_timeout_driver(Some(
        ReceiveTimeoutDriverShared::new(CountingDriver::new(driver_calls.clone(), factory_calls.clone())),
      ));

    let config: ActorSystemConfig<TestRuntime> = ActorSystemConfig::default();

    let mut system: ActorSystem<u32, _, AlwaysRestart> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
    spawn_test_actor(&mut system);

    assert_eq!(driver_calls.load(Ordering::SeqCst), 1);
    assert_eq!(factory_calls.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn actor_system_prefers_bundle_factory_over_driver() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let driver_calls = Arc::new(AtomicUsize::new(0));
    let driver_factory_calls = Arc::new(AtomicUsize::new(0));
    let bundle_factory_calls = Arc::new(AtomicUsize::new(0));

    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime.clone())
      .with_receive_timeout_driver(Some(ReceiveTimeoutDriverShared::new(CountingDriver::new(
        driver_calls.clone(),
        driver_factory_calls.clone(),
      ))))
      .with_receive_timeout_factory(ReceiveTimeoutFactoryShared::new(CountingFactory::new(
        bundle_factory_calls.clone(),
      )));

    let config: ActorSystemConfig<TestRuntime> = ActorSystemConfig::default();

    let mut system: ActorSystem<u32, _, AlwaysRestart> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
    spawn_test_actor(&mut system);

    assert_eq!(bundle_factory_calls.load(Ordering::SeqCst), 1);
    assert_eq!(driver_calls.load(Ordering::SeqCst), 0);
    assert_eq!(driver_factory_calls.load(Ordering::SeqCst), 0);
  }

  #[test]
  fn actor_system_prefers_config_factory_over_driver_and_bundle() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let driver_calls = Arc::new(AtomicUsize::new(0));
    let driver_factory_calls = Arc::new(AtomicUsize::new(0));
    let bundle_factory_calls = Arc::new(AtomicUsize::new(0));
    let config_factory_calls = Arc::new(AtomicUsize::new(0));

    let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_runtime.clone())
      .with_receive_timeout_driver(Some(ReceiveTimeoutDriverShared::new(CountingDriver::new(
        driver_calls.clone(),
        driver_factory_calls.clone(),
      ))))
      .with_receive_timeout_factory(ReceiveTimeoutFactoryShared::new(CountingFactory::new(
        bundle_factory_calls.clone(),
      )));

    let config: ActorSystemConfig<TestRuntime> = ActorSystemConfig::default().with_receive_timeout_factory(Some(
      ReceiveTimeoutFactoryShared::new(CountingFactory::new(config_factory_calls.clone())),
    ));

    let mut system: ActorSystem<u32, _, AlwaysRestart> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
    spawn_test_actor(&mut system);

    assert_eq!(config_factory_calls.load(Ordering::SeqCst), 1);
    assert_eq!(bundle_factory_calls.load(Ordering::SeqCst), 0);
    assert_eq!(driver_calls.load(Ordering::SeqCst), 0);
    assert_eq!(driver_factory_calls.load(Ordering::SeqCst), 0);
  }
}

use cellex_utils_core_rs::sync::ArcShared;
use core::any::Any;
use core::sync::atomic::{AtomicUsize, Ordering};
use futures::executor::block_on;
use futures::future;

#[derive(Debug)]
struct CounterExtension {
  id: ExtensionId,
  hits: AtomicUsize,
}

impl CounterExtension {
  fn new() -> Self {
    Self {
      id: next_extension_id(),
      hits: AtomicUsize::new(0),
    }
  }

  fn extension_id(&self) -> ExtensionId {
    self.id
  }

  fn increment(&self) {
    let _ = self.hits.fetch_add(1, Ordering::SeqCst);
  }

  fn value(&self) -> usize {
    self.hits.load(Ordering::SeqCst)
  }
}

impl Extension for CounterExtension {
  fn extension_id(&self) -> ExtensionId {
    self.id
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[cfg(target_has_atomic = "ptr")]
type NoopDispatchFn = dyn Fn(DynMessage, i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type NoopDispatchFn = dyn Fn(DynMessage, i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>;

fn noop_sender<M>() -> MessageSender<M, ThreadSafe>
where
  M: Element, {
  let dispatch_impl: Arc<NoopDispatchFn> =
    Arc::new(|_message: DynMessage, _priority: i8| -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> { Ok(()) });
  let dispatch = ArcShared::from_arc_for_testing_dont_use_production(dispatch_impl);
  let internal = InternalMessageSender::new(dispatch);
  MessageSender::new(internal)
}

#[test]
fn test_supervise_builder_sets_strategy() {
  let props = Props::with_behavior(|| {
    Behaviors::supervise(Behavior::stateless(
      |_: &mut Context<'_, '_, u32, GenericActorRuntime<TestMailboxRuntime>>, _: u32| Ok(()),
    ))
    .with_strategy(SupervisorStrategy::Restart)
  });
  let (_, supervisor_cfg) = props.into_parts();
  assert_eq!(
    supervisor_cfg,
    SupervisorStrategyConfig::from_strategy(SupervisorStrategy::Restart)
  );
}

#[test]
#[ignore = "panic handling for supervised restarts/stops not yet fully wired"]
fn test_supervise_stop_on_failure() {
  let failures: StdArc<Mutex<Vec<String>>> = StdArc::new(Mutex::new(Vec::new()));
  let failures_clone = StdArc::clone(&failures);
  let listener = FailureEventListener::new(move |event: FailureEvent| {
    let FailureEvent::RootEscalated(info) = event;
    failures_clone.lock().unwrap().push(info.description().into_owned());
  });

  let actor_runtime = GenericActorRuntime::new(TestMailboxRuntime::unbounded());
  let config = ActorSystemConfig::default().with_failure_event_listener(Some(listener));
  let mut system: ActorSystem<u32, _, AlwaysRestart> = ActorSystem::new_with_actor_runtime(actor_runtime, config);

  let props = Props::with_behavior(|| {
    Behaviors::supervise(Behaviors::receive(|_, _: u32| panic!("boom"))).with_strategy(SupervisorStrategy::Stop)
  });
  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.tell(1).expect("send message");
  let panic_result = catch_unwind(AssertUnwindSafe(|| {
    block_on(root.dispatch_next()).expect("dispatch failure");
  }));
  assert!(panic_result.is_err(), "expected actor to panic under Stop strategy");

  assert!(failures.lock().unwrap().iter().any(|reason| reason.contains("boom")));
}

#[test]
#[ignore = "panic handling for supervised restarts/resume not yet fully wired"]
fn test_supervise_resume_on_failure() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(mailbox_runtime), ActorSystemConfig::default());

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));

  let props = Props::with_behavior({
    let log_factory = log.clone();
    move || {
      let log_clone = log_factory.clone();
      Behaviors::supervise(Behaviors::receive(move |_, msg: u32| {
        if msg == 0 {
          panic!("fail once");
        }
        log_clone.borrow_mut().push(msg);
        Ok(Behaviors::same())
      }))
      .with_strategy(SupervisorStrategy::Resume)
    }
  });
  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.tell(0).expect("send failure message");
  let panic_result = catch_unwind(AssertUnwindSafe(|| {
    block_on(root.dispatch_next()).expect("dispatch failure");
  }));
  assert!(panic_result.is_err(), "expected actor to panic before resume");

  actor_ref.tell(42).expect("send second message");
  block_on(root.dispatch_next()).expect("process resume");

  assert_eq!(log.borrow().as_slice(), &[42]);
}

#[test]
fn typed_actor_system_handles_user_messages() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(mailbox_runtime), ActorSystemConfig::default());

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let props = Props::new(move |_, msg: u32| {
    log_clone.borrow_mut().push(msg);
    Ok(())
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");
  actor_ref.tell(11).expect("tell");
  block_on(root.dispatch_next()).expect("dispatch");

  assert_eq!(log.borrow().as_slice(), &[11]);
}

fn spawn_actor_with_counter_extension<R>(
  actor_runtime: R,
) -> (
  ActorSystem<u32, R, AlwaysRestart>,
  ExtensionId,
  ArcShared<CounterExtension>,
)
where
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  let extension = CounterExtension::new();
  let extension_id = extension.extension_id();
  let extension_handle = ArcShared::new(extension);
  let extension_probe = extension_handle.clone();

  let config = ActorSystemConfig::default().with_extension_handle(extension_handle);
  let system: ActorSystem<u32, R, AlwaysRestart> = ActorSystem::new_with_actor_runtime(actor_runtime, config);
  (system, extension_id, extension_probe)
}

#[test]
fn actor_context_accesses_registered_extension() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let (mut system, extension_id, extension_probe) = spawn_actor_with_counter_extension(actor_runtime);
  let mut root = system.root_context();
  assert_eq!(
    root.extension::<CounterExtension, _, _>(extension_id, |ext| ext.value()),
    Some(0)
  );

  let props = Props::with_behavior(move || {
    Behaviors::receive(
      move |ctx: &mut Context<'_, '_, u32, GenericActorRuntime<TestMailboxRuntime>>, msg: u32| {
        let _ = msg;
        ctx
          .extension::<CounterExtension, _, _>(extension_id, |ext| {
            ext.increment();
          })
          .expect("extension registered");
        Ok(Behaviors::same())
      },
    )
  });

  let actor_ref = root.spawn(props).expect("spawn actor");
  actor_ref.tell(42).expect("tell message");
  block_on(root.dispatch_next()).expect("dispatch message");

  assert_eq!(extension_probe.value(), 1);
  assert_eq!(
    system.extension::<CounterExtension, _, _>(extension_id, |ext| ext.value()),
    Some(1)
  );
}

#[test]
fn serializer_extension_provides_json_roundtrip() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let (system, _, _) = spawn_actor_with_counter_extension(actor_runtime);

  #[derive(Debug, Serialize, Deserialize, PartialEq)]
  struct JsonPayload {
    number: u32,
  }

  let roundtrip = system
    .extensions()
    .with::<SerializerRegistryExtension, _, _>(serializer_extension_id(), |ext| {
      let serializer = ext
        .registry()
        .get(SERDE_JSON_SERIALIZER_ID)
        .expect("serde json registered");
      let payload = JsonPayload { number: 7 };
      let encoded = serde_json::to_vec(&payload).expect("encode json");
      let message = serializer
        .serialize_with_type_name(encoded.as_slice(), "JsonPayload")
        .expect("serialize message");
      let decoded = serializer.deserialize(&message).expect("deserialize message");
      serde_json::from_slice::<JsonPayload>(&decoded).expect("decode json")
    })
    .expect("serializer extension available");

  assert_eq!(roundtrip, JsonPayload { number: 7 });
}

#[test]
fn test_typed_actor_handles_system_stop() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let stopped: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
  let stopped_clone = stopped.clone();

  let system_handler = move |_: &mut Context<'_, '_, u32, _>, sys_msg: SystemMessage| {
    if matches!(sys_msg, SystemMessage::Stop) {
      *stopped_clone.borrow_mut() = true;
    }
  };

  let props = Props::with_system_handler(move |_, _msg: u32| Ok(()), Some(system_handler));

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");
  actor_ref.send_system(SystemMessage::Stop).expect("send stop");
  block_on(root.dispatch_next()).expect("dispatch");

  assert!(*stopped.borrow(), "SystemMessage::Stop should be handled");
}

#[test]
fn user_message_releases_metadata_on_drop() {
  let metadata = MessageMetadata::<ThreadSafe>::new().with_sender(noop_sender::<ParentMessage>());
  let envelope = MessageEnvelope::user_with_metadata(ParentMessage("ping".into()), metadata);
  let key = match &envelope {
    MessageEnvelope::User(user) => user.metadata_key().expect("metadata key expected"),
    MessageEnvelope::System(_) => unreachable!(),
  };
  drop(envelope);
  assert!(
    take_metadata::<ThreadSafe>(key).is_none(),
    "metadata key should be released on drop"
  );
}

#[test]
fn metadata_key_consumed_once() {
  let metadata = MessageMetadata::<ThreadSafe>::new().with_sender(noop_sender::<ParentMessage>());
  let envelope = MessageEnvelope::user_with_metadata(ParentMessage("pong".into()), metadata);
  let key = match envelope {
    MessageEnvelope::User(user) => user.into_parts().1.expect("metadata key expected"),
    MessageEnvelope::System(_) => unreachable!(),
  };
  assert!(take_metadata::<ThreadSafe>(key).is_some());
  assert!(take_metadata::<ThreadSafe>(key).is_none());
}

#[test]
fn test_typed_actor_handles_watch_unwatch() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(mailbox_runtime), ActorSystemConfig::default());

  let watchers_count: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
  let watchers_count_clone = watchers_count.clone();

  let system_handler = Some(
    |ctx: &mut Context<'_, '_, u32, _>, sys_msg: SystemMessage| match sys_msg {
      SystemMessage::Watch(watcher) => {
        ctx.register_watcher(watcher);
      }
      SystemMessage::Unwatch(watcher) => {
        ctx.unregister_watcher(watcher);
      }
      _ => {}
    },
  );

  let props = Props::with_behavior_and_system(
    {
      let watchers_factory = watchers_count_clone.clone();
      move || {
        let watchers_clone = watchers_factory.clone();
        Behavior::stateless(move |ctx: &mut Context<'_, '_, u32, _>, _msg: u32| {
          *watchers_clone.borrow_mut() = ctx.watchers().len();
          Ok(())
        })
      }
    },
    system_handler,
  );

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");

  // Get initial watcher count (parent is automatically registered)
  actor_ref.tell(1).expect("tell");
  block_on(root.dispatch_next()).expect("dispatch initial");
  let initial_count = *watchers_count.borrow();

  let watcher_id = ActorId(999);
  actor_ref
    .send_system(SystemMessage::Watch(watcher_id))
    .expect("send watch");
  block_on(root.dispatch_next()).expect("dispatch watch");

  actor_ref.tell(2).expect("tell");
  block_on(root.dispatch_next()).expect("dispatch user message");

  let after_watch_count = *watchers_count.borrow();
  assert_eq!(
    after_watch_count,
    initial_count + 1,
    "Watcher count should increase by 1"
  );

  actor_ref
    .send_system(SystemMessage::Unwatch(watcher_id))
    .expect("send unwatch");
  block_on(root.dispatch_next()).expect("dispatch unwatch");

  actor_ref.tell(3).expect("tell");
  block_on(root.dispatch_next()).expect("dispatch user message");

  let after_unwatch_count = *watchers_count.borrow();
  assert_eq!(
    after_unwatch_count, initial_count,
    "Watcher count should return to initial"
  );
}

#[cfg(feature = "std")]
#[test]
fn test_typed_actor_stateful_behavior_with_system_message() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  // Stateful behavior: count user messages and track system messages
  let count = Rc::new(RefCell::new(0u32));
  let failures = Rc::new(RefCell::new(0u32));

  let failures_clone = failures.clone();
  let system_handler = move |_ctx: &mut Context<'_, '_, u32, _>, sys_msg: SystemMessage| {
    if matches!(sys_msg, SystemMessage::Suspend) {
      *failures_clone.borrow_mut() += 1;
    }
  };

  let props = Props::with_behavior_and_system(
    {
      let count_factory = count.clone();
      move || {
        let count_clone = count_factory.clone();
        Behavior::stateless(move |_ctx: &mut Context<'_, '_, u32, _>, msg: u32| {
          *count_clone.borrow_mut() += msg;
          Ok(())
        })
      }
    },
    Some(system_handler),
  );

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");

  // Send user messages
  actor_ref.tell(10).expect("tell 10");
  block_on(root.dispatch_next()).expect("dispatch user 1");

  actor_ref.tell(5).expect("tell 5");
  block_on(root.dispatch_next()).expect("dispatch user 2");

  // Send system message (Suspend doesn't stop the actor)
  actor_ref.send_system(SystemMessage::Suspend).expect("send suspend");
  block_on(root.dispatch_next()).expect("dispatch system");

  // Verify stateful behavior updated correctly
  assert_eq!(*count.borrow(), 15, "State should accumulate user messages");
  assert_eq!(*failures.borrow(), 1, "State should track system messages");
}

#[test]
fn test_behaviors_receive_self_loop() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));

  let props = Props::with_behavior({
    let log_factory = log.clone();
    move || {
      let log_clone = log_factory.clone();
      Behaviors::receive(move |ctx: &mut Context<'_, '_, u32, _>, msg: u32| {
        log_clone.borrow_mut().push(msg);
        if msg < 2 {
          ctx.self_ref().tell(msg + 1).expect("self tell");
        }
        Ok(Behaviors::same())
      })
    }
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.tell(0).expect("tell initial");
  block_on(root.dispatch_next()).expect("process 0");
  block_on(root.dispatch_next()).expect("process 1");
  block_on(root.dispatch_next()).expect("process 2");

  assert_eq!(log.borrow().as_slice(), &[0, 1, 2]);
}

#[test]
fn test_behaviors_receive_message_without_context() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));

  let props = Props::with_behavior({
    let log_clone = log.clone();
    move || {
      let log_inner = log_clone.clone();
      Behaviors::receive_message(move |msg: u32| {
        log_inner.borrow_mut().push(msg);
        Ok(Behaviors::same())
      })
    }
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.tell(7).expect("tell first");
  block_on(root.dispatch_next()).expect("process first");

  actor_ref.tell(8).expect("tell second");
  block_on(root.dispatch_next()).expect("process second");

  assert_eq!(log.borrow().as_slice(), &[7, 8]);
}

#[test]
fn test_parent_spawns_child_with_distinct_message_type() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<ParentMessage, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let child_log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
  let child_log_for_parent = child_log.clone();

  let props = Props::with_behavior({
    let child_log_factory = child_log_for_parent.clone();
    move || {
      let child_log_parent = child_log_factory.clone();
      Behaviors::receive(move |ctx: &mut Context<'_, '_, ParentMessage, _>, msg: ParentMessage| {
        let name = msg.0;
        let child_props = Props::with_behavior({
          let child_log_factory = child_log_parent.clone();
          move || {
            let child_log_for_child = child_log_factory.clone();
            Behaviors::receive(
              move |_child_ctx: &mut Context<'_, '_, ChildMessage, _>, child_msg: ChildMessage| {
                child_log_for_child.borrow_mut().push(child_msg.text.clone());
                Ok(Behaviors::same())
              },
            )
          }
        });
        let child_ref = ctx.spawn_child(child_props);
        child_ref
          .tell(ChildMessage {
            text: format!("hello {name}"),
          })
          .expect("tell child");
        Ok(Behaviors::same())
      })
    }
  });
  let mut root = system.root_context();
  let parent_ref = root.spawn(props).expect("spawn parent");

  parent_ref
    .tell(ParentMessage("world".to_string()))
    .expect("tell parent");
  block_on(root.dispatch_next()).expect("dispatch parent");
  block_on(root.dispatch_next()).expect("dispatch child");

  assert_eq!(child_log.borrow().as_slice(), &["hello world".to_string()]);
}

#[test]
fn test_message_adapter_converts_external_message() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let adapter_slot: Rc<RefCell<Option<MessageAdapterRef<String, u32, _>>>> = Rc::new(RefCell::new(None));

  let props = Props::with_behavior({
    let log_factory = log.clone();
    let adapter_slot_factory = adapter_slot.clone();
    move || {
      let log_clone = log_factory.clone();
      let adapter_slot_clone = adapter_slot_factory.clone();
      Behaviors::receive(move |ctx: &mut Context<'_, '_, u32, _>, msg: u32| {
        log_clone.borrow_mut().push(msg);
        if adapter_slot_clone.borrow().is_none() {
          let adapter = ctx.message_adapter(|text: String| text.len() as u32);
          adapter_slot_clone.borrow_mut().replace(adapter);
        }
        Ok(Behaviors::same())
      })
    }
  });
  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.tell(1).expect("initial message");
  block_on(root.dispatch_next()).expect("dispatch primary");

  let adapter = adapter_slot.borrow().as_ref().cloned().expect("adapter must exist");
  adapter.tell("abcd".to_string()).expect("adapter tell");
  block_on(root.dispatch_next()).expect("dispatch adapted");

  assert_eq!(log.borrow().as_slice(), &[1, 4]);
}

#[test]
fn test_parent_actor_spawns_child() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let child_log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let child_log_for_parent = child_log.clone();
  let child_ref_holder: Rc<RefCell<Option<ActorRef<u32, _>>>> = Rc::new(RefCell::new(None));
  let child_ref_holder_clone = child_ref_holder.clone();

  let props = Props::with_behavior({
    let child_log_factory = child_log_for_parent.clone();
    let child_ref_holder_factory = child_ref_holder_clone.clone();
    move || {
      let child_log_parent = child_log_factory.clone();
      let child_ref_holder_local = child_ref_holder_factory.clone();
      Behavior::stateless(move |ctx: &mut Context<'_, '_, u32, _>, msg: u32| {
        if child_ref_holder_local.borrow().is_none() {
          let child_log_for_child = child_log_parent.clone();
          let child_props = Props::new(move |_, child_msg: u32| {
            child_log_for_child.borrow_mut().push(child_msg);
            Ok(())
          });
          let child_ref = ctx.spawn_child(child_props);
          child_ref_holder_local.borrow_mut().replace(child_ref);
        }

        if let Some(child_ref) = child_ref_holder_local.borrow().clone() {
          child_ref.tell(msg * 2).expect("tell child");
        }
        Ok(())
      })
    }
  });

  let mut root = system.root_context();
  let parent_ref = root.spawn(props).expect("spawn parent actor");

  parent_ref.tell(3).expect("tell parent");
  block_on(root.dispatch_next()).expect("dispatch parent");
  block_on(root.dispatch_next()).expect("dispatch child");

  assert_eq!(child_log.borrow().as_slice(), &[6]);
}

#[test]
fn test_behaviors_setup_spawns_named_child() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<String, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let child_log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

  let props = Props::with_behavior({
    let child_log_factory = child_log.clone();
    move || {
      let child_log_parent = child_log_factory.clone();
      Behaviors::setup(move |ctx| {
        let child_props = Props::with_behavior({
          let child_log_clone = child_log_parent.clone();
          move || {
            let log_ref = child_log_clone.clone();
            Behavior::stateless(move |_, msg: String| {
              log_ref.borrow_mut().push(msg);
              Ok(())
            })
          }
        });
        let greeter = ctx.spawn_child(child_props);
        Ok(Behaviors::receive(move |_, msg: String| {
          greeter.tell(format!("hello {msg}")).expect("forward to child");
          Ok(Behaviors::same())
        }))
      })
    }
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.tell("world".to_string()).expect("tell message");
  block_on(root.dispatch_next()).expect("dispatch setup+message");
  block_on(root.dispatch_next()).expect("dispatch child");

  assert_eq!(child_log.borrow().as_slice(), &["hello world".to_string()]);
}

#[test]
fn test_receive_signal_post_stop() {
  let mailbox_runtime = TestMailboxRuntime::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_runtime);
  let mut system: ActorSystem<u32, _, AlwaysRestart> =
    ActorSystem::new_with_actor_runtime(actor_runtime, ActorSystemConfig::default());

  let signals: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
  let signals_clone = signals.clone();

  let props = Props::with_behavior(move || {
    let signals_cell = signals_clone.clone();
    Behaviors::receive(|_, msg: u32| {
      if msg == 0 {
        Ok(Behaviors::stopped())
      } else {
        Ok(Behaviors::same())
      }
    })
    .receive_signal(move |_, signal| {
      match signal {
        Signal::PostStop => signals_cell.borrow_mut().push("post_stop"),
      }
      Behaviors::same()
    })
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn actor");

  actor_ref.send_system(SystemMessage::Stop).expect("send stop");
  block_on(root.dispatch_next()).expect("dispatch stop");
  let _ = block_on(root.dispatch_next());

  assert_eq!(signals.borrow().as_slice(), &["post_stop"]);
}

fn noop_waker() -> Waker {
  fn clone(_: *const ()) -> RawWaker {
    noop_raw_waker()
  }
  fn wake(_: *const ()) {}
  fn wake_by_ref(_: *const ()) {}
  fn drop(_: *const ()) {}

  fn noop_raw_waker() -> RawWaker {
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    RawWaker::new(core::ptr::null(), &VTABLE)
  }

  unsafe { Waker::from_raw(noop_raw_waker()) }
}

fn resolve<F>(mut future: F) -> F::Output
where
  F: Future + Unpin, {
  let waker = noop_waker();
  let mut future = Pin::new(&mut future);
  let mut cx = TaskContext::from_waker(&waker);
  loop {
    match future.as_mut().poll(&mut cx) {
      Poll::Ready(value) => return value,
      Poll::Pending => core::hint::spin_loop(),
    }
  }
}

#[test]
fn ask_future_completes_successfully() {
  let (future, responder) = create_ask_handles::<u32, ThreadSafe>();
  responder.dispatch_user(7_u32).expect("dispatch succeeds");

  let result = resolve(future);
  assert_eq!(result.expect("ask result"), 7);
}

#[test]
fn ask_future_timeout_returns_error() {
  let (future, _responder) = create_ask_handles::<u32, ThreadSafe>();
  let timed = ask_with_timeout(future, future::ready(()));

  let result = resolve(timed);
  assert!(
    matches!(result, Err(AskError::Timeout)),
    "unexpected result: {:?}",
    result
  );
}

#[test]
fn ask_future_responder_drop_propagates() {
  let (future, responder) = create_ask_handles::<u32, ThreadSafe>();
  drop(responder);

  let result = resolve(future);
  assert!(matches!(result, Err(AskError::ResponderDropped)));
}

#[test]
fn ask_future_cancelled_on_drop() {
  let (future, responder) = create_ask_handles::<u32, ThreadSafe>();
  drop(future);
  drop(responder);
}

mod metrics_injection {
  use super::*;
  use crate::internal::mailbox::test_support::TestMailboxRuntime;
  use crate::internal::scheduler::{ActorScheduler, SchedulerBuilder, SchedulerSpawnContext};
  use crate::{
    ActorSystem, ActorSystemConfig, DynMessage, FailureTelemetryShared, GenericActorRuntime, MailboxRuntime,
    MetricsEvent, MetricsSink, MetricsSinkShared, Supervisor, TelemetryObservationConfig,
  };
  use alloc::boxed::Box;
  use core::marker::PhantomData;
  use std::sync::{Arc, Mutex};

  #[derive(Clone)]
  struct TaggedSink {
    _id: &'static str,
  }

  impl MetricsSink for TaggedSink {
    fn record(&self, _event: MetricsEvent) {
      let _ = self._id;
    }
  }

  struct RecordingScheduler<M, R> {
    metrics: Arc<Mutex<Option<usize>>>,
    _marker: PhantomData<(M, R)>,
  }

  impl<M, R> RecordingScheduler<M, R> {
    fn new(metrics: Arc<Mutex<Option<usize>>>) -> Self {
      Self {
        metrics,
        _marker: PhantomData,
      }
    }
  }

  fn make_scheduler_builder(metrics: Arc<Mutex<Option<usize>>>) -> SchedulerBuilder<DynMessage, TestMailboxRuntime> {
    SchedulerBuilder::new(move |_runtime, _extensions| {
      Box::new(RecordingScheduler::<DynMessage, TestMailboxRuntime>::new(
        metrics.clone(),
      ))
    })
  }

  #[async_trait::async_trait(?Send)]
  impl<M, R> ActorScheduler<M, R> for RecordingScheduler<M, R>
  where
    M: Element,
    R: MailboxRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<M>>: Clone,
    R::Signal: Clone,
  {
    fn spawn_actor(
      &mut self,
      _supervisor: Box<dyn Supervisor<M>>,
      _context: SchedulerSpawnContext<M, R>,
    ) -> Result<crate::internal::context::InternalActorRef<M, R>, SpawnError<M>> {
      Err(SpawnError::Queue(QueueError::Disconnected))
    }

    fn set_receive_timeout_factory(&mut self, _factory: Option<crate::ReceiveTimeoutFactoryShared<M, R>>) {}

    fn set_root_event_listener(&mut self, _listener: Option<crate::FailureEventListener>) {}

    fn set_root_escalation_handler(&mut self, _handler: Option<crate::FailureEventHandler>) {}

    fn set_root_failure_telemetry(&mut self, _telemetry: FailureTelemetryShared) {}

    fn set_root_observation_config(&mut self, _config: TelemetryObservationConfig) {}

    fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
      let mut slot = self.metrics.lock().unwrap();
      *slot = sink.map(|shared| shared.with_ref(|inner| inner as *const _ as *const () as usize));
    }

    fn set_parent_guardian(
      &mut self,
      _control_ref: crate::internal::context::InternalActorRef<M, R>,
      _map_system: MapSystemShared<M>,
    ) {
    }

    fn on_escalation(
      &mut self,
      _handler: Box<dyn FnMut(&crate::FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
    ) {
    }

    fn take_escalations(&mut self) -> Vec<crate::FailureInfo> {
      Vec::new()
    }

    fn actor_count(&self) -> usize {
      0
    }

    fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
      Ok(false)
    }

    async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
      Ok(())
    }
  }

  #[test]
  fn actor_system_prefers_config_metrics_sink_over_bundle() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let recorded = Arc::new(Mutex::new(None));
    let recorded_clone = recorded.clone();

    let runtime_sink = MetricsSinkShared::new(TaggedSink { _id: "runtime" });
    let config_sink = MetricsSinkShared::new(TaggedSink { _id: "config" });
    let config_ptr = config_sink.with_ref(|inner| inner as *const _ as *const () as usize);

    let actor_runtime = GenericActorRuntime::new(mailbox_runtime.clone())
      .with_scheduler_builder(make_scheduler_builder(recorded_clone.clone()))
      .with_metrics_sink_shared(runtime_sink);

    let config: ActorSystemConfig<GenericActorRuntime<TestMailboxRuntime>> =
      ActorSystemConfig::default().with_metrics_sink_shared(config_sink);

    let _system =
      ActorSystem::<DynMessage, GenericActorRuntime<TestMailboxRuntime>>::new_with_actor_runtime(actor_runtime, config);

    assert_eq!(*recorded.lock().unwrap(), Some(config_ptr));
  }

  #[test]
  fn actor_system_uses_bundle_metrics_when_config_absent() {
    let mailbox_runtime = TestMailboxRuntime::unbounded();
    let recorded = Arc::new(Mutex::new(None));
    let recorded_clone = recorded.clone();

    let runtime_sink = MetricsSinkShared::new(TaggedSink { _id: "runtime" });
    let runtime_ptr = runtime_sink.with_ref(|inner| inner as *const _ as *const () as usize);

    let actor_runtime = GenericActorRuntime::new(mailbox_runtime)
      .with_scheduler_builder(make_scheduler_builder(recorded_clone.clone()))
      .with_metrics_sink_shared(runtime_sink);

    let config: ActorSystemConfig<GenericActorRuntime<TestMailboxRuntime>> = ActorSystemConfig::default();

    let _system =
      ActorSystem::<DynMessage, GenericActorRuntime<TestMailboxRuntime>>::new_with_actor_runtime(actor_runtime, config);

    assert_eq!(*recorded.lock().unwrap(), Some(runtime_ptr));
  }
}
