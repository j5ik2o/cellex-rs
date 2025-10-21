use alloc::boxed::Box;
use core::{convert::Infallible, future::Future, pin::Pin};

use cellex_utils_core_rs::{sync::ArcShared, Element, QueueError};

use crate::api::{
  actor::{root_context::RootContext, shutdown_token::ShutdownToken},
  actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
  actor_scheduler::ready_queue_scheduler::ReadyQueueWorker,
  guardian::GuardianStrategy,
  mailbox::messages::PriorityEnvelope,
  messaging::AnyMessage,
};

/// 共通のアクターシステムインタフェース。
///
/// ランタイム駆動やテスト用ヘルパから必要とされる最小限の操作を公開する。
pub trait ActorSystem<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: GuardianStrategy<MailboxOf<AR>>, {
  /// システム全体のシャットダウントークンを取得する。
  fn shutdown_token(&self) -> ShutdownToken;

  /// ルートコンテキストを借用する。
  fn root_context(&mut self) -> RootContext<'_, U, AR, Strat>;

  /// ReadyQueue のワーカハンドルを取得する。
  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>>;

  /// ReadyQueue によるスケジューリングをサポートしているか判定する。
  fn supports_ready_queue(&self) -> bool;

  /// キューが空になるまで同期的にメッセージを処理する。
  fn run_until_idle(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>;

  /// 条件が満たされるまでメッセージディスパッチを継続する。
  fn run_until<'a, F>(
    &'a mut self,
    should_continue: F,
  ) -> Pin<Box<dyn Future<Output = Result<(), QueueError<PriorityEnvelope<AnyMessage>>>> + 'a>>
  where
    F: FnMut() -> bool + 'a;

  /// 明示的に停止されるまでメッセージディスパッチを継続する。
  fn run_forever(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>> + '_>>;

  /// 次のメッセージを一件処理する。
  fn dispatch_next(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = Result<(), QueueError<PriorityEnvelope<AnyMessage>>>> + '_>>;
}
