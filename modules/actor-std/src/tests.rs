use core::time::Duration;
use std::sync::{Arc, Mutex};

use cellex_actor_core_rs::{
  actor_loop,
  api::{
    actor::{actor_context::ActorContext, ActorId, ChildNaming, Props},
    actor_runtime::GenericActorRuntime,
    actor_scheduler::ActorSchedulerSpawnContext,
    actor_system::{map_system::MapSystemShared, ActorSystem, ActorSystemConfig, Spawn},
    extensions::Extensions,
    mailbox::{messages::SystemMessage, MailboxOptions},
    messaging::{AnyMessage, MessageEnvelope},
    process::{
      pid::{Pid, SystemId},
      process_registry::ProcessRegistry,
    },
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    supervision::supervisor::NoopSupervisor,
  },
};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_std_rs::{sync::ArcStateCell, StateCell};
use spin::RwLock;

use super::*;

type TestResult<T = ()> = Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
  System(SystemMessage),
}

async fn run_test_actor_loop_updates_state() -> TestResult {
  let (mailbox, sender) = TokioMailbox::new(8);
  let mailbox = Arc::new(mailbox);
  let state = ArcStateCell::new(0_u32);

  let actor_state = state.clone();
  let actor_mailbox = mailbox.clone();

  let spawner = TokioSpawner;

  spawner.spawn(async move {
    let timer = TokioTimer;
    actor_loop(actor_mailbox.as_ref(), &timer, move |msg: u32| {
      let mut guard = actor_state.borrow_mut();
      *guard += msg;
    })
    .await;
  });

  sender.send(4_u32).map_err(|err| format!("send message: {:?}", err))?;
  tokio::time::sleep(Duration::from_millis(10)).await;

  assert_eq!(*state.borrow(), 4_u32);
  Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_actor_loop_updates_state() -> TestResult {
  run_test_actor_loop_updates_state().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_actor_loop_updates_state_multi_thread() -> TestResult {
  run_test_actor_loop_updates_state().await
}

async fn run_typed_actor_system_handles_user_messages() -> TestResult {
  let mut system: ActorSystem<u32, _> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(TokioMailboxRuntime), ActorSystemConfig::default());

  let log: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
  let log_clone = log.clone();

  let props = Props::new(move |_, msg: u32| {
    log_clone.lock().unwrap_or_else(|err| err.into_inner()).push(msg);
    Ok(())
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).map_err(|err| format!("spawn typed actor: {:?}", err))?;

  actor_ref.tell(99).map_err(|err| format!("tell: {:?}", err))?;
  root.dispatch_next().await.map_err(|err| format!("dispatch next: {:?}", err))?;

  assert_eq!(log.lock().unwrap_or_else(|err| err.into_inner()).as_slice(), &[99]);
  Ok(())
}

async fn run_receive_timeout_triggers() -> TestResult {
  let mailbox_factory = TokioMailboxRuntime;
  let mut config: ActorSystemConfig<TokioActorRuntime> = ActorSystemConfig::default();
  config.set_receive_timeout_scheduler_factory_shared_opt(Some(ReceiveTimeoutSchedulerFactoryShared::new(
    TokioReceiveTimeoutSchedulerFactory::new(),
  )));
  let mut system: ActorSystem<u32, _> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(mailbox_factory), config);

  let timeout_log: Arc<Mutex<Vec<SystemMessage>>> = Arc::new(Mutex::new(Vec::new()));
  let props = Props::with_system_handler(
    move |ctx: &mut ActorContext<'_, '_, u32, TokioActorRuntime>, msg| {
      if msg == 1 {
        ctx.set_receive_timeout(Duration::from_millis(10));
      }
      Ok(())
    },
    Some({
      let timeout_clone = timeout_log.clone();
      move |_: &mut ActorContext<'_, '_, u32, TokioActorRuntime>, sys: SystemMessage| {
        if matches!(sys, SystemMessage::ReceiveTimeout) {
          timeout_clone.lock().unwrap_or_else(|err| err.into_inner()).push(sys);
        }
      }
    }),
  );

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).map_err(|err| format!("spawn receive-timeout actor: {:?}", err))?;

  actor_ref.tell(1).map_err(|err| format!("tell: {:?}", err))?;
  root.dispatch_next().await.map_err(|err| format!("dispatch user: {:?}", err))?;

  tokio::time::sleep(Duration::from_millis(30)).await;
  root.dispatch_next().await.map_err(|err| format!("dispatch timeout: {:?}", err))?;

  let log = timeout_log.lock().unwrap_or_else(|err| err.into_inner());
  assert!(!log.is_empty(), "ReceiveTimeout が少なくとも 1 回は発火する想定");
  assert!(
    log.iter().all(|sys| matches!(sys, SystemMessage::ReceiveTimeout)),
    "ReceiveTimeout 以外のシグナルは届かない想定"
  );
  Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn typed_actor_system_handles_user_messages() -> TestResult {
  run_typed_actor_system_handles_user_messages().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_actor_system_handles_user_messages_multi_thread() -> TestResult {
  run_typed_actor_system_handles_user_messages().await
}

#[tokio::test]
async fn tokio_scheduler_builder_dispatches() -> TestResult {
  let bundle: TokioActorRuntime = tokio_actor_runtime();
  let mailbox_factory = bundle.mailbox_factory().clone();
  let mut scheduler = tokio_scheduler_builder().build(mailbox_factory.clone(), Extensions::new());

  let log: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
  let log_clone = log.clone();

  let mailbox_factory_shared = ArcShared::new(mailbox_factory.clone());
  let process_registry = ArcShared::new(ProcessRegistry::new(SystemId::new("tokio-test"), None));
  let pid_slot = ArcShared::new(RwLock::new(None::<Pid>));
  let context = ActorSchedulerSpawnContext {
    mailbox_factory: mailbox_factory.clone(),
    mailbox_factory_shared,
    map_system: MapSystemShared::new(|sys| AnyMessage::new(MessageEnvelope::<Message>::System(sys))),
    mailbox_options: MailboxOptions::default(),
    handler: Box::new(move |_, msg: AnyMessage| {
      if let Ok(MessageEnvelope::System(system)) = msg.downcast::<MessageEnvelope<Message>>() {
        log_clone.lock().unwrap_or_else(|err| err.into_inner()).push(Message::System(system));
      }
      Ok(())
    }),
    child_naming: ChildNaming::Auto,
    process_registry,
    actor_pid_slot: pid_slot,
  };

  scheduler.spawn_actor(Box::new(NoopSupervisor), context).map_err(|err| format!("spawn actor: {:?}", err))?;

  scheduler.dispatch_next().await.map_err(|err| format!("dispatch next: {:?}", err))?;

  assert_eq!(log.lock().unwrap_or_else(|err| err.into_inner()).as_slice(), &[Message::System(SystemMessage::Watch(
    ActorId::ROOT
  ))]);
  Ok(())
}

#[test]
fn tokio_bundle_sets_default_receive_timeout_factory() {
  let bundle: TokioActorRuntime = tokio_actor_runtime();
  let factory_from_bundle = bundle.receive_timeout_scheduler_factory_shared();
  let factory_from_driver = bundle.receive_timeout_scheduler_factory_provider_shared_opt();
  assert!(
    factory_from_bundle.is_some() || factory_from_driver.is_some(),
    "Tokio バンドルは ReceiveTimeout ドライバまたはファクトリを提供する想定"
  );
}

#[tokio::test(flavor = "current_thread")]
async fn receive_timeout_triggers() -> TestResult {
  run_receive_timeout_triggers().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn receive_timeout_triggers_multi_thread() -> TestResult {
  run_receive_timeout_triggers().await
}
