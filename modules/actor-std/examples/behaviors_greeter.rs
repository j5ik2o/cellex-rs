//! Minimal greeting actor example that uses the Behaviors DSL.
//!
//! `Behaviors::setup` initializes state (number of greetings) and `Behaviors::receive`
//! defines how each command is handled.

use cellex_actor_core_rs::api::{
  actor::{behavior::Behaviors, Props},
  actor_runtime::GenericActorRuntime,
  actor_system::{GenericActorSystem, GenericActorSystemConfig},
};
use cellex_actor_std_rs::TokioMailboxRuntime;
use tracing_subscriber::FmtSubscriber;

#[derive(Clone, Debug)]
enum Command {
  Greet(String),
  Report,
  Stop,
}

fn main() {
  // tracing サブスクライバを初期化（既に設定済みなら無視）
  let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
  let _ = FmtSubscriber::builder().with_env_filter(env_filter).try_init();

  let mut system: GenericActorSystem<Command, _> = GenericActorSystem::new_with_actor_runtime(
    GenericActorRuntime::new(TokioMailboxRuntime),
    GenericActorSystemConfig::default(),
  );
  let mut root = system.root_context();

  let greeter_props = Props::with_behavior(|| {
    Behaviors::setup(move |ctx| {
      let mut greeted = 0usize;
      let logger = ctx.log();
      let actor_id = ctx.actor_id();

      Ok(Behaviors::receive_message(move |msg: Command| match msg {
        | Command::Greet(name) => {
          greeted += 1;
          logger.info(|| format!("actor {:?} says: Hello, {}!", actor_id, name));
          Ok(Behaviors::same())
        },
        | Command::Report => {
          logger.info(|| format!("actor {:?} greeted {} people", actor_id, greeted));
          Ok(Behaviors::same())
        },
        | Command::Stop => {
          logger.warn(|| format!("actor {:?} is stopping after {} greetings", actor_id, greeted));
          Ok(Behaviors::stopped())
        },
      }))
    })
  });

  let greeter = root.spawn(greeter_props).expect("spawn greeter");

  greeter.tell(Command::Greet("Alice".to_owned())).expect("greet Alice");
  greeter.tell(Command::Greet("Bob".to_owned())).expect("greet Bob");
  greeter.tell(Command::Report).expect("report greetings");
  greeter.tell(Command::Stop).expect("stop greeter");

  system.run_until_idle().expect("drain mailbox");
}
