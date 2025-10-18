use core::time::Duration;
use std::sync::{Arc, Mutex};

use cellex_actor_core_rs::{
  actor_loop,
  api::{
    actor::{context::Context, ActorId, ChildNaming, Props},
    actor_runtime::GenericActorRuntime,
    actor_system::{map_system::MapSystemShared, ActorSystem, ActorSystemConfig, Spawn},
    extensions::Extensions,
    mailbox::{MailboxOptions, SystemMessage},
    receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
    scheduler::SchedulerSpawnContext,
    supervision::supervisor::NoopSupervisor,
  },
};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_std_rs::{ArcStateCell, StateCell};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
  System(SystemMessage),
}

async fn run_test_actor_loop_updates_state() {
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

  sender.send(4_u32).expect("send message");
  tokio::time::sleep(Duration::from_millis(10)).await;

  assert_eq!(*state.borrow(), 4_u32);
}

#[tokio::test(flavor = "current_thread")]
async fn test_actor_loop_updates_state() {
  run_test_actor_loop_updates_state().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_actor_loop_updates_state_multi_thread() {
  run_test_actor_loop_updates_state().await;
}

async fn run_typed_actor_system_handles_user_messages() {
  let mut system: ActorSystem<u32, _> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(TokioMailboxRuntime), ActorSystemConfig::default());

  let log: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
  let log_clone = log.clone();

  let props = Props::new(move |_, msg: u32| {
    log_clone.lock().unwrap().push(msg);
    Ok(())
  });

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn typed actor");

  actor_ref.tell(99).expect("tell");
  root.dispatch_next().await.expect("dispatch next");

  assert_eq!(log.lock().unwrap().as_slice(), &[99]);
}

async fn run_receive_timeout_triggers() {
  let mailbox_factory = TokioMailboxRuntime;
  let mut config: ActorSystemConfig<TokioActorRuntime> = ActorSystemConfig::default();
  config.set_receive_timeout_scheduler_factory_shared_opt(Some(ReceiveTimeoutSchedulerFactoryShared::new(
    TokioReceiveTimeoutSchedulerFactory::new(),
  )));
  let mut system: ActorSystem<u32, _> =
    ActorSystem::new_with_actor_runtime(GenericActorRuntime::new(mailbox_factory), config);

  let timeout_log: Arc<Mutex<Vec<SystemMessage>>> = Arc::new(Mutex::new(Vec::new()));
  let props = Props::with_system_handler(
    move |ctx: &mut Context<'_, '_, u32, TokioActorRuntime>, msg| {
      if msg == 1 {
        ctx.set_receive_timeout(Duration::from_millis(10));
      }
      Ok(())
    },
    Some({
      let timeout_clone = timeout_log.clone();
      move |_: &mut Context<'_, '_, u32, TokioActorRuntime>, sys: SystemMessage| {
        if matches!(sys, SystemMessage::ReceiveTimeout) {
          timeout_clone.lock().unwrap().push(sys);
        }
      }
    }),
  );

  let mut root = system.root_context();
  let actor_ref = root.spawn(props).expect("spawn receive-timeout actor");

  actor_ref.tell(1).expect("tell");
  root.dispatch_next().await.expect("dispatch user");

  tokio::time::sleep(Duration::from_millis(30)).await;
  root.dispatch_next().await.expect("dispatch timeout");

  let log = timeout_log.lock().unwrap();
  assert!(!log.is_empty(), "ReceiveTimeout が少なくとも 1 回は発火する想定");
  assert!(
    log.iter().all(|sys| matches!(sys, SystemMessage::ReceiveTimeout)),
    "ReceiveTimeout 以外のシグナルは届かない想定"
  );
}

#[tokio::test(flavor = "current_thread")]
async fn typed_actor_system_handles_user_messages() {
  run_typed_actor_system_handles_user_messages().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_actor_system_handles_user_messages_multi_thread() {
  run_typed_actor_system_handles_user_messages().await;
}

#[tokio::test]
async fn tokio_scheduler_builder_dispatches() {
  let bundle: TokioActorRuntime = tokio_actor_runtime();
  let mailbox_factory = bundle.mailbox_factory().clone();
  let mut scheduler = tokio_scheduler_builder().build(mailbox_factory.clone(), Extensions::new());

  let log: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
  let log_clone = log.clone();

  let mailbox_factory_shared = ArcShared::new(mailbox_factory.clone());
  let context = SchedulerSpawnContext {
    mailbox_factory:        mailbox_factory.clone(),
    mailbox_factory_shared: mailbox_factory_shared,
    map_system:             MapSystemShared::new(Message::System),
    mailbox_options:        MailboxOptions::default(),
    handler:                Box::new(move |_, msg: Message| {
      log_clone.lock().unwrap().push(msg);
      Ok(())
    }),
    child_naming:           ChildNaming::Auto,
  };

  scheduler.spawn_actor(Box::new(NoopSupervisor), context).unwrap();

  scheduler.dispatch_next().await.unwrap();

  assert_eq!(log.lock().unwrap().as_slice(), &[Message::System(SystemMessage::Watch(ActorId::ROOT))]);
}

#[test]
fn tokio_bundle_sets_default_receive_timeout_factory() {
  let bundle: TokioActorRuntime = tokio_actor_runtime();
  let factory_from_bundle = bundle.receive_timeout_factory();
  let factory_from_driver = bundle.receive_timeout_driver_factory();
  assert!(
    factory_from_bundle.is_some() || factory_from_driver.is_some(),
    "Tokio バンドルは ReceiveTimeout ドライバまたはファクトリを提供する想定"
  );
}

#[tokio::test(flavor = "current_thread")]
async fn receive_timeout_triggers() {
  run_receive_timeout_triggers().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn receive_timeout_triggers_multi_thread() {
  run_receive_timeout_triggers().await;
}
