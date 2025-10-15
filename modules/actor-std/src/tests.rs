use super::*;
use cellex_actor_core_rs::MailboxOptions;
use cellex_actor_core_rs::{
  actor_loop, ActorId, ActorSystem, Context, Extensions, MailboxHandleFactoryStub, MapSystemShared, NoopSupervisor,
  Props, RuntimeEnv, SchedulerSpawnContext, Spawn, StateCell, SystemMessage,
};
use core::time::Duration;
use std::sync::{Arc, Mutex};

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
  let factory = TokioMailboxRuntime;
  let mut system: ActorSystem<u32, _> = ActorSystem::new(factory);

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
  let factory = TokioMailboxRuntime;
  let mut config: ActorSystemConfig<RuntimeEnv<TokioMailboxRuntime>> = ActorSystemConfig::default();
  config.set_receive_timeout_factory(Some(
    ReceiveTimeoutFactoryShared::new(TokioReceiveTimeoutSchedulerFactory::new()).for_runtime_bundle(),
  ));
  let mut system: ActorSystem<u32, _> = ActorSystem::new_with_config(factory, config);

  let timeout_log: Arc<Mutex<Vec<SystemMessage>>> = Arc::new(Mutex::new(Vec::new()));
  let props = Props::with_system_handler(
    move |ctx: &mut Context<'_, '_, u32, RuntimeEnv<TokioMailboxRuntime>>, msg| {
      if msg == 1 {
        ctx.set_receive_timeout(Duration::from_millis(10));
      }
      Ok(())
    },
    Some({
      let timeout_clone = timeout_log.clone();
      move |_: &mut Context<'_, '_, u32, RuntimeEnv<TokioMailboxRuntime>>, sys: SystemMessage| {
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
  let runtime = RuntimeEnv::new(TokioMailboxRuntime);
  let mut scheduler = tokio_scheduler_builder().build(runtime.clone(), Extensions::new());

  let log: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
  let log_clone = log.clone();

  let mailbox_handle_factory_stub = MailboxHandleFactoryStub::from_runtime(runtime.clone());
  let context = SchedulerSpawnContext {
    runtime,
    mailbox_handle_factory_stub,
    map_system: MapSystemShared::new(Message::System),
    mailbox_options: MailboxOptions::default(),
    handler: Box::new(move |_, msg: Message| {
      log_clone.lock().unwrap().push(msg);
      Ok(())
    }),
  };

  scheduler.spawn_actor(Box::new(NoopSupervisor), context).unwrap();

  scheduler.dispatch_next().await.unwrap();

  assert_eq!(
    log.lock().unwrap().as_slice(),
    &[Message::System(SystemMessage::Watch(ActorId::ROOT))]
  );
}

#[test]
fn tokio_bundle_sets_default_receive_timeout_factory() {
  let bundle = RuntimeEnv::new(TokioMailboxRuntime).with_tokio_scheduler();
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
