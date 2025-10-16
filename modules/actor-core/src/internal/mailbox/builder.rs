use crate::internal::mailbox::traits::{MailboxHandle, MailboxPair, MailboxProducer, MailboxRuntime, MailboxSignal};
use crate::internal::mailbox::MailboxOptions;
use crate::PriorityEnvelope;
use cellex_utils_core_rs::Element;

/// Builder abstraction specialised for priority mailboxes.
///
/// このトレイトは優先度付きメールボックスを生成する責務を `MailboxRuntime`
/// から切り出し、スケジューラ層が具象ファクトリ型へ直接依存しないようにする。
pub trait PriorityMailboxBuilder<M>: Clone
where
  M: Element, {
  /// Mailbox が利用するシグナル型。
  type Signal: MailboxSignal;
  /// 優先度付きメッセージを保持する Mailbox 型。
  type Mailbox: MailboxHandle<PriorityEnvelope<M>, Signal = Self::Signal> + Clone;
  /// メールボックスへメッセージを投入するプロデューサ型。
  type Producer: MailboxProducer<PriorityEnvelope<M>> + Clone;

  /// 指定されたオプションでメールボックスを生成する。
  fn build_priority_mailbox(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox, Self::Producer>;

  /// 既定設定でメールボックスを生成する。
  fn build_default_priority_mailbox(&self) -> MailboxPair<Self::Mailbox, Self::Producer> {
    self.build_priority_mailbox(MailboxOptions::default())
  }
}

impl<M, R> PriorityMailboxBuilder<M> for R
where
  M: Element,
  R: MailboxRuntime + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  type Mailbox = R::Mailbox<PriorityEnvelope<M>>;
  type Producer = R::Producer<PriorityEnvelope<M>>;
  type Signal = R::Signal;

  fn build_priority_mailbox(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox, Self::Producer> {
    MailboxRuntime::build_mailbox::<PriorityEnvelope<M>>(self, options)
  }

  fn build_default_priority_mailbox(&self) -> MailboxPair<Self::Mailbox, Self::Producer> {
    MailboxRuntime::build_default_mailbox::<PriorityEnvelope<M>>(self)
  }
}
