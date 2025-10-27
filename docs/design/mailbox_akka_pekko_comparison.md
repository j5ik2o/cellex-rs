# Akka/Pekko メールボックス機能との包括的比較

**作成日**: 2025-10-27
**対象**: cellex-rs vs Akka/Pekko vs protoactor-go
**評価**: メールボックス機能の同等性評価

---

## エグゼクティブサマリー

cellex-rs のメールボックス実装は、**Akka/Pekko および protoactor-go の基本機能を実装済み**です。2025-10-27 に ActorCell レベルで Suspend/Resume が追加され、ユーザーメッセージの停止と Resume 後の再開が動作確認できました。基本的なメッセージング、優先度制御、オーバーフロー処理、DeadLetter は完全に実装されています。

**欠けている主要機能**:
- **Stashing**（メッセージの一時保留・再配置）
- **ControlAwareMailbox**（制御メッセージの自動優先化）- ただし SystemMessage の優先度で部分的に実現

**リファクタリングプランでの対応**:
- Suspend/Resume 実装（Phase 0〜2B）
- Throughput 制限（Phase 1〜3）
- Middleware チェイン（Phase 2B）
- Observability Hub（Phase 3）

**総合評価**: ⭐⭐⭐⭐☆ (4.0/5.0) - Stashing 等の拡張で 5.0 に到達

---

## 1. Akka/Pekko の全メールボックス機能

### 1.1 基本メールボックスタイプ

| タイプ | Akka/Pekko | 説明 | 用途 |
|-------|-----------|------|-----|
| **Unbounded** | ✅ | 無制限容量、メモリ制限まで受付 | デフォルト、低負荷環境 |
| SingleConsumerOnlyUnboundedMailbox | ✅ | MPSC キュー、最速 | デフォルト実装 |
| UnboundedMailbox | ✅ | ConcurrentLinkedQueue ベース | マルチディスパッチャ |
| **Bounded** | ✅ | 容量制限あり、満杯時の動作設定可能 | メモリ保護、バックプレッシャ |
| NonBlockingBoundedMailbox | ✅ | オーバーフロー時 DeadLetter へ | 推奨される bounded 実装 |
| BoundedMailbox | ✅ | オーバーフロー時ブロック | レガシー、非推奨 |

### 1.2 優先度付きメールボックス

| タイプ | Akka/Pekko | 説明 | 用途 |
|-------|-----------|------|-----|
| **UnboundedPriorityMailbox** | ✅ | PriorityBlockingQueue ベース | 優先度順処理 |
| **UnboundedStablePriorityMailbox** | ✅ | 同優先度で FIFO 保証 | 決定的な順序が必要 |
| **BoundedPriorityMailbox** | ✅ | 容量制限 + 優先度 | メモリ保護 + 優先度 |
| **BoundedStablePriorityMailbox** | ✅ | 容量制限 + 安定優先度 | 完全な順序保証 |

### 1.3 制御メッセージ対応

| タイプ | Akka/Pekko | 説明 | 用途 |
|-------|-----------|------|-----|
| **UnboundedControlAwareMailbox** | ✅ | ControlMessage を自動優先化 | システム制御 |
| **BoundedControlAwareMailbox** | ✅ | 容量制限 + 制御優先 | メモリ保護 + 制御 |

### 1.4 高度な機能

| 機能 | Akka/Pekko | 説明 |
|-----|-----------|------|
| **Stashing** | ✅ | メッセージを一時保留し、後で再配置 |
| **Throughput 制限** | ✅ | 1アクターあたりの最大処理数（公平性） |
| **DeadLetter** | ✅ | 配信不能メッセージの処理 |
| **Mailbox サイズ監視** | ✅ | キュー長のメトリクス |
| **カスタムメールボックス** | ✅ | MailboxType 継承で独自実装 |

---

## 2. protoactor-go のメールボックス機能

| 機能 | protoactor-go | 説明 |
|-----|--------------|------|
| **User/System キュー分離** | ✅ | systemMailbox (MPSC) + userMailbox (queue interface) |
| **MessageInvoker** | ✅ | InvokeUserMessage / InvokeSystemMessage |
| **MailboxMiddleware** | ✅ | MailboxStarted / MessagePosted / MessageReceived / MailboxEmpty |
| **Suspend/Resume** | ✅ | SuspendMailbox / ResumeMailbox システムメッセージ |
| **Dispatcher 連携** | ✅ | dispatcher.Schedule() + Throughput() |
| **スケジューリング制御** | ✅ | idle/running 状態（atomic CAS） |
| **エスカレーション** | ✅ | invoker.EscalateFailure() |
| **Batching** | ✅ | MessageBatch の展開 |

---

## 3. cellex-rs の実装状況

### 3.1 実装済み機能の詳細

#### ✅ Bounded/Unbounded メールボックス

**実装箇所**: `modules/actor-core/src/api/mailbox/queue_mailbox.rs`

```rust
pub struct QueueMailbox<Q, S> {
  pub(super) core: QueueMailboxCore<Q, S>,
}
```

- **QueueSize 抽象**:
  - `QueueSize::Limited(n)`: bounded（容量制限）
  - `QueueSize::Limitless`: unbounded（無制限）
- **柔軟なキュー実装**:
  - `SyncMailboxQueue`: 同期版
  - 任意の `MailboxQueue<M>` trait 実装を差し替え可能

**Akka/Pekko との比較**:
- ✅ SingleConsumerOnlyUnboundedMailbox 相当: `QueueMailbox<UnboundedQueue, _>`
- ✅ BoundedMailbox 相当: `QueueMailbox<BoundedQueue, _>`
- ✅ NonBlockingBoundedMailbox 相当: `MailboxOverflowPolicy::DropNewest` で実現

#### ✅ 優先度付きメールボックス

**実装箇所**: `modules/actor-core/src/api/mailbox/messages/priority_envelope.rs`

```rust
pub struct PriorityEnvelope<M> {
    priority: i8,  // -128 〜 127（8段階以上）
    message: M,
}
```

- **SystemMessage の優先度**:
  ```rust
  SystemMessage::Failure(_) => DEFAULT_PRIORITY + 12,
  SystemMessage::Restart => DEFAULT_PRIORITY + 11,
  SystemMessage::Suspend | Resume => DEFAULT_PRIORITY + 9,
  SystemMessage::Escalate(_) => DEFAULT_PRIORITY + 13,
  SystemMessage::ReceiveTimeout => DEFAULT_PRIORITY + 8,
  ```

**Akka/Pekko との比較**:
- ✅ UnboundedPriorityMailbox 相当: `PriorityEnvelope` + unbounded queue
- ✅ BoundedPriorityMailbox 相当: `PriorityEnvelope` + bounded queue
- ⚠️ UnboundedStablePriorityMailbox: 同優先度の FIFO 保証は未確認（要調査）

**cellex-rs の優位性**:
- Akka/Pekko: 優先度は暗黙的（ControlMessage インターフェース）
- cellex-rs: 明示的な `i8` 型で 256 段階の細かい制御

#### ✅ オーバーフロー処理

**実装箇所**: `modules/actor-core/src/api/mailbox/mailbox_overflow_policy.rs`

```rust
pub enum MailboxOverflowPolicy {
    DropNewest,   // 新しいメッセージを破棄
    DropOldest,   // 古いメッセージを破棄
    Grow,         // 動的に拡張
    Block,        // 送信者をブロック
}
```

**Akka/Pekko との比較**:
- ✅ NonBlockingBoundedMailbox (DropNewest): `MailboxOverflowPolicy::DropNewest`
- ✅ BoundedMailbox (Block): `MailboxOverflowPolicy::Block`
- ✅ DropOldest: Akka にはない cellex-rs 独自機能
- ✅ Grow: 動的拡張（Akka の Unbounded に相当）

#### ✅ DeadLetter

**実装箇所**: `modules/actor-core/src/api/process/dead_letter.rs`

```rust
pub struct DeadLetter {
    pub pid: Pid,
    pub message: AnyMessage,
    pub reason: DeadLetterReason,
}

pub enum DeadLetterReason {
    Stopped,
    Terminated,
    Timeout,
    QueueFull,
    // ...
}
```

**Akka/Pekko との比較**:
- ✅ DeadLetter 機能は完全実装
- ✅ DeadLetterHub でサブスクリプション可能
- ✅ DeadLetterReason で詳細な原因分類

#### ✅ User/System メッセージ分離

**実装箇所**: `ActorCell::process_envelopes()`

```rust
// ActorCell が SystemMessage を優先処理
if let Some(SystemMessage::Escalate(failure)) = envelope.system_message() {
    // システムメッセージの即時処理
}
```

**Akka/Pekko との比較**:
- ✅ protoactor-go の systemMailbox / userMailbox に相当
- ✅ Akka の ControlAwareMailbox に部分的に相当（優先度ベース）

#### ✅ Suspend/Resume

**実装箇所**: `modules/actor-core/src/api/mailbox/messages/system_message.rs`

```rust
pub enum SystemMessage {
    Suspend,
    Resume,
    // ...
}
```

**Akka/Pekko との比較**:
- ✅ protoactor-go の SuspendMailbox / ResumeMailbox に相当
- ✅ Akka の Stash とは異なる概念（後述）

#### ✅ メトリクスとスケジューラ統合

**実装箇所**: `modules/actor-core/src/api/mailbox.rs`

```rust
pub trait Mailbox<M> {
    fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {}
    fn set_scheduler_hook(&mut self, hook: Option<ReadyQueueHandle>) {}
}
```

**Akka/Pekko との比較**:
- ✅ Akka の mailbox size monitoring に相当
- ✅ ReadyQueueHandle で Coordinator への自動通知

---

### 3.2 部分的実装または計画中の機能

#### ⚠️ Throughput 制限（Phase 1〜3 で実装予定）

**現状**: `ReadyQueueCoordinator` トレイトに `throughput_hint()` が存在

```rust
pub trait ReadyQueueCoordinator: Send + Sync {
    fn throughput_hint(&self) -> usize;
}
```

**リファクタリングプラン**（セクション 4.7）:
> 処理ループは `throughput_hint` を参照し、指定件数に達したら自発的に `InvokeResult::Yielded` を返すことで公平性を担保する。

**Akka/Pekko との比較**:
- Akka のデフォルト: throughput = 100（1アクターあたり最大100メッセージ処理後に次のアクターへ）
- ⚠️ cellex-rs: トレイト定義はあるが、実装の完全性は未確認

#### ⚠️ MailboxMiddleware（Phase 2B で実装予定）

**リファクタリングプラン**（セクション 4.4）:

```rust
pub trait MiddlewareChain {
    fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()>;
    fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult);
}
```

**Akka/Pekko との比較**:
- protoactor-go: MailboxStarted / MessagePosted / MessageReceived / MailboxEmpty
- cellex-rs 計画: before_invoke / after_invoke（より汎用的）

---

### 3.3 未実装の主要機能

#### ❌ Stashing（最も重要な欠落機能）

**Akka/Pekko での実装**:

```scala
class MyActor extends Actor with Stash {
  def receive = {
    case Initialize =>
      // 初期化中
      unstashAll()
      context.become(ready)
    case other =>
      stash()  // 初期化完了まで保留
  }

  def ready: Receive = {
    case msg => // 通常処理
  }
}
```

**機能説明**:
- メッセージを一時的に stash（保留）し、後で unstash（再配置）
- アクターの状態遷移時に未処理メッセージを保留
- 初期化完了後、接続確立後など、準備が整うまでメッセージを待機

**技術的要件**（Pekko ドキュメントより）:
- Deque-based mailbox が必須（`UnboundedDequeBasedMailbox`）
- Priority mailbox との併用は非推奨（stash 後は優先度が失われる）

**cellex-rs での実現方法（提案）**:

```rust
pub trait ActorBehavior {
    fn stash(&mut self) -> Result<(), StashError>;
    fn unstash_all(&mut self) -> Result<(), StashError>;
}

pub struct StashBuffer<M> {
    buffer: VecDeque<M>,
    max_capacity: Option<usize>,
}
```

**実装優先度**: 🔴 **高**（Akka/Pekko の重要パターン）

#### ❌ ControlAwareMailbox（部分的に実現）

**Akka/Pekko での実装**:

```scala
trait ControlMessage  // マーカートレイト

case class Priority() extends ControlMessage
case class UserMsg()  // 通常メッセージ

// UnboundedControlAwareMailbox が ControlMessage を自動優先化
```

**cellex-rs での現状**:
- ✅ `SystemMessage` が固定優先度を持つ（部分的に実現）
- ❌ ユーザー定義の制御メッセージに自動優先度を付与する仕組みはない

**cellex-rs での実現方法（提案）**:

```rust
pub trait ControlMessage: Message {}

impl<M: ControlMessage> PriorityEnvelope<M> {
    pub fn new_control(message: M) -> Self {
        Self::new_with_priority(message, CONTROL_PRIORITY)
    }
}
```

**実装優先度**: 🟡 **中**（SystemMessage で代替可能）

---

## 4. 機能カバレッジマトリクス

### 4.1 基本メールボックス機能

| 機能 | Akka/Pekko | protoactor-go | cellex-rs 現状 | リファクタ後 | 備考 |
|-----|-----------|--------------|---------------|-------------|-----|
| **Unbounded Mailbox** | ✅ | ✅ | ✅ | ✅ | 完全同等 |
| **Bounded Mailbox** | ✅ | ❌ | ✅ | ✅ | Go は言語的に不要 |
| **Priority Mailbox** | ✅ | ❌ | ✅ | ✅ | cellex-rs の方が柔軟（8段階 vs 2段階） |
| **Stable Priority** | ✅ | ❌ | ❓ | ❓ | 同優先度の FIFO 保証（要確認） |
| **DeadLetter** | ✅ | ❌ | ✅ | ✅ | 詳細な理由分類 |

### 4.2 高度な機能

| 機能 | Akka/Pekko | protoactor-go | cellex-rs 現状 | リファクタ後 | 備考 |
|-----|-----------|--------------|---------------|-------------|-----|
| **Stashing** | ✅ | ❌ | ❌ | ❓ | **最重要の欠落** |
| **Throughput 制限** | ✅ | ✅ | ⚠️ | ✅ | トレイト定義済み、実装確認中 |
| **ControlAware** | ✅ | ❌ | ⚠️ | ⚠️ | SystemMessage で部分的に実現 |
| **Mailbox Middleware** | ❌ | ✅ | ❌ | ✅ | Phase 2B で実装予定 |
| **Suspend/Resume** | ✅ | ✅ | ✅ | ✅ | ActorCell がユーザーメッセージ停止/再開を制御 |

### 4.3 メトリクスと監視

| 機能 | Akka/Pekko | protoactor-go | cellex-rs 現状 | リファクタ後 | 備考 |
|-----|-----------|--------------|---------------|-------------|-----|
| **Mailbox サイズ監視** | ✅ | ❌ | ⚠️ | ✅ | Observability Hub (Phase 3) |
| **レイテンシ統計** | ❌ | ❌ | ❌ | ❓ | nexus-actor-rs にあり |
| **サスペンション統計** | ❌ | ❌ | ❌ | ❓ | nexus-actor-rs にあり |
| **メトリクスシンク統合** | ⚠️ | ❌ | ✅ | ✅ | cellex-rs が先進的 |

### 4.4 カスタマイズ性

| 機能 | Akka/Pekko | protoactor-go | cellex-rs 現状 | リファクタ後 | 備考 |
|-----|-----------|--------------|---------------|-------------|-----|
| **カスタムメールボックス** | ✅ | ✅ | ✅ | ✅ | ジェネリックな抽象化 |
| **オーバーフロー戦略** | ⚠️ | ❌ | ✅ | ✅ | DropOldest は cellex-rs 独自 |
| **シグナル抽象** | ❌ | ❌ | ✅ | ✅ | Tokio/Embassy 統合 |

---

## 5. 総合評価

### 5.1 実装レベル評価

| 評価軸 | スコア | コメント |
|-------|------|---------|
| **基本メッセージング** | ⭐⭐⭐⭐⭐ (5.0) | Akka/Pekko と完全同等、一部優位（優先度の柔軟性） |
| **スケジューリング** | ⭐⭐⭐⭐☆ (4.0) | Throughput 実装確認が必要 |
| **拡張機能** | ⭐⭐⭐☆☆ (3.0) | Stashing 欠落が大きい |
| **メトリクス** | ⭐⭐⭐⭐☆ (4.0) | Observability Hub で改善見込み |
| **カスタマイズ性** | ⭐⭐⭐⭐⭐ (5.0) | ジェネリック抽象化が秀逸 |
| **総合** | **⭐⭐⭐⭐☆ (4.5/5.0)** | **Stashing 追加で 5.0 到達** |

### 5.2 cellex-rs の強み

1. **より柔軟な優先度制御**
   - Akka/Pekko: 2段階（ControlMessage vs 通常）
   - cellex-rs: `i8` 型で 256 段階

2. **ジェネリックな抽象化**
   - 任意のキュー実装を差し替え可能（`MailboxQueue<M>` trait）
   - 任意のシグナル実装（Tokio / Embassy / テスト環境）

3. **詳細なオーバーフロー戦略**
   - DropNewest / DropOldest / Grow / Block
   - Akka/Pekko: Block または Drop のみ

4. **メトリクスシンク統合**
   - `set_metrics_sink()` で柔軟な計測
   - Akka/Pekko: 限定的な監視機能

5. **DeadLetter の詳細な理由分類**
   - `DeadLetterReason` で原因分析が容易

### 5.3 cellex-rs の弱み（改善が必要）

1. **Suspend/Resume の周辺機能が未整備** ⚠️
   - メトリクス収集やバックプレッシャ連携が未実装
   - Resume 通知のメトリクス、サスペンション統計が未整備
   - 旧実装（nexus-actor-rs）の `MailboxSuspensionMetrics` を参考に拡張余地あり

2. **Stashing の欠落** 🔴
   - Akka/Pekko の重要パターンが使えない
   - 状態遷移時のメッセージ保留ができない
   - **Phase 2B〜3 で実装すべき**

3. **Throughput 実装の完全性が不明** ⚠️
   - トレイト定義はあるが、実装確認が必要
   - リファクタリングプラン（Phase 1〜3）で明確化

4. **ControlAwareMailbox の自動化** ⚠️
   - ユーザー定義の制御メッセージに自動優先度を付与する仕組みがない
   - SystemMessage で代替可能だが、柔軟性に欠ける

5. **Stable Priority の未確認** ⚠️
   - 同優先度のメッセージの FIFO 保証が不明
   - UnboundedStablePriorityMailbox 相当の機能確認が必要

---

## 6. 推奨事項

### 6.1 即座に実施すべき改善（優先度: 🔴 高）

#### 推奨 1: Stashing 機能の実装

**問題**: Akka/Pekko の最も重要なパターンの一つが欠落。

**解決策**: Phase 2B〜3 で Stashing を実装：

```rust
// 1. Stash トレイトの定義
pub trait Stashable: Actor {
    fn stash(&mut self, ctx: &mut Context) -> Result<(), StashError>;
    fn unstash_all(&mut self, ctx: &mut Context) -> Result<(), StashError>;
    fn unstash(&mut self, ctx: &mut Context) -> Result<(), StashError>;
}

// 2. StashBuffer の実装
pub struct StashBuffer<M> {
    buffer: VecDeque<M>,
    max_capacity: Option<usize>,
}

// 3. Deque-based mailbox の要件
// - QueueMailbox<VecDeque<M>, S> で実現可能（既存実装で対応可能）
```

**実装フェーズ**: Phase 2B または Phase 3
**影響範囲**: `ActorContext`、`MessageInvoker`
**Akka/Pekko ドキュメント参考**: [Stash - Akka Documentation](https://doc.akka.io/docs/akka/current/actors.html#stash)

#### 推奨 2: Throughput 実装の確認と完成

**問題**: `throughput_hint()` の実装が完全か不明。

**解決策**:
1. `DefaultReadyQueueCoordinator` の `throughput_hint()` 実装を確認
2. `MessageInvoker` が `throughput_hint()` を参照して `InvokeResult::Yielded` を返すロジックの実装確認
3. 不足があれば Phase 1 で完成

**検証方法**:
```rust
// MessageInvoker の実装例
pub fn invoke_batch(&mut self, max_messages: usize) -> InvokeResult {
    let throughput = self.coordinator.throughput_hint();
    let mut processed = 0;

    for envelope in self.mailbox.dequeue_batch(max_messages) {
        self.handle(envelope);
        processed += 1;

        if processed >= throughput {
            return InvokeResult::Yielded;  // 公平性のため yield
        }
    }

    InvokeResult::Completed { ready_hint: self.mailbox.has_more() }
}
```

### 6.2 中期的に検討すべき改善（優先度: 🟡 中）

#### 推奨 3: ControlAware メカニズムの拡張

**問題**: ユーザー定義の制御メッセージに自動優先度を付与できない。

**解決策**: マーカートレイトとマクロによる自動優先度付与：

```rust
// 1. ControlMessage トレイト
pub trait ControlMessage: Message {}

// 2. #[derive(ControlMessage)] マクロ
#[derive(Message, ControlMessage)]
struct MyControlMsg;

// 3. PriorityEnvelope での自動優先度
impl<M: ControlMessage> From<M> for PriorityEnvelope<M> {
    fn from(msg: M) -> Self {
        Self::new_with_priority(msg, CONTROL_PRIORITY)
    }
}
```

**実装フェーズ**: Phase 3〜4
**影響範囲**: `message-derive` マクロ、`PriorityEnvelope`

#### 推奨 4: Stable Priority の実装確認と検証

**問題**: 同優先度のメッセージの FIFO 保証が不明。

**解決策**:
1. 現状の `VecDeque` + `sort_by_key` 実装が安定ソートか確認
2. 不安定な場合、`sort_by_key` → `stable_sort_by_key` に変更
3. テストケースで FIFO 保証を検証

**検証テストケース**:
```rust
#[test]
fn test_stable_priority() {
    let mailbox = create_priority_mailbox();

    mailbox.send(msg("A", priority: 1));
    mailbox.send(msg("B", priority: 1));
    mailbox.send(msg("C", priority: 1));

    assert_eq!(mailbox.recv(), "A");  // FIFO 保証
    assert_eq!(mailbox.recv(), "B");
    assert_eq!(mailbox.recv(), "C");
}
```

### 6.3 長期的に検討すべき改善（優先度: 🟢 低）

#### 推奨 5: レイテンシヒストグラムとサスペンション統計

nexus-actor-rs の機能を Phase 3 の Observability Hub に統合：
- 17バケットのレイテンシヒストグラム
- Suspend/Resume の頻度と継続時間の統計

#### 推奨 6: カスタムメールボックスのドキュメント整備

ユーザーが独自のメールボックスを実装するためのガイド作成：
- `MailboxQueue<M>` trait の実装方法
- `MailboxSignal` の実装方法
- カスタムオーバーフロー戦略の実装例

---

## 7. リファクタリングプランへの追加提案

### 7.1 `actor_scheduler_refactor.md` への追加セクション

**追加箇所**: セクション 4「目標アーキテクチャ」に以下を追加：

```markdown
### 4.13 Stashing サポート（Phase 2B〜3）

Akka/Pekko の Stashing パターンをサポートし、アクターの状態遷移時にメッセージを保留・再配置できるようにする。

#### 設計方針
- `Stashable` トレイトによる opt-in 方式
- `StashBuffer` による VecDeque ベースの実装
- 容量制限とオーバーフロー戦略の設定可能化
- MessageInvoker との統合（`before_invoke` で stash 判定）

#### API 設計案
\`\`\`rust
pub trait Stashable: Actor {
    fn stash(&mut self, ctx: &mut Context) -> Result<(), StashError>;
    fn unstash_all(&mut self, ctx: &mut Context) -> Result<(), StashError>;
}

pub struct StashBuffer<M> {
    buffer: VecDeque<M>,
    max_capacity: Option<usize>,
}
\`\`\`

#### 実装フェーズ
- Phase 2B: `Stashable` トレイトと `StashBuffer` の実装
- Phase 3: MessageInvoker との統合、テスト、ドキュメント
```

### 7.2 Phase 別タスクへの追加

**Phase 2B**:
- [ ] `Stashable` トレイト定義
- [ ] `StashBuffer<M>` 実装（VecDeque ベース）
- [ ] `ActorContext` への `stash()` / `unstash_all()` メソッド追加
- [ ] 単体テスト（10 ケース以上）

**Phase 3**:
- [ ] MessageInvoker との統合
- [ ] stash 中のメッセージのメトリクス収集
- [ ] Observability Hub での stash サイズ監視
- [ ] 統合テスト（5 シナリオ）
- [ ] ドキュメントとサンプルコード

---

## 8. 結論

### 8.1 質問への最終回答

> **質問**: メールボックス機能が protoactor-go, Akka/Pekko と同等レベルか？
>
> **回答**: ✅ **基本機能は同等水準に達しています。**

**カバー状況**:
- ✅ **基本メッセージング**: 完全同等（Bounded/Unbounded、Priority、DeadLetter）
- ✅ **Suspend/Resume**: ActorCell がユーザーメッセージの停止/再開を制御
- ⚠️ **スケジューリング**: Throughput 実装の完全性確認が必要
- ⚠️ **拡張機能**: Stashing は未対応で大きなギャップ
- ✅ **メトリクス**: リファクタリング後に Akka/Pekko を超える可能性

**cellex-rs の優位点**:
- より柔軟な優先度制御（256 段階 vs 2 段階）
- ジェネリックな抽象化（キュー、シグナル）
- 詳細なオーバーフロー戦略（DropOldest など独自機能）

**cellex-rs の課題**:
- ⚠️ **Suspend/Resume のメトリクス・バックプレッシャ連携**（拡張余地）
- 🔴 **Stashing の欠落**（Phase 2B〜3 で実装すべき）
- ⚠️ Throughput 実装の完全性確認
- ⚠️ ControlAwareMailbox の自動化

### 8.2 総合評価

| 比較対象 | cellex-rs 現状 | Stashing 実装後 |
|---------|---------------|----------------|
| **Akka/Pekko** | ⭐⭐⭐⭐☆ (4.0/5.0) | ⭐⭐⭐⭐⭐ (5.0/5.0) |
| **protoactor-go** | ⭐⭐⭐⭐⭐ (5.0/5.0) | ⭐⭐⭐⭐⭐ (5.0/5.0) |

**現状の課題**:
- Stashing が未対応
- Suspend/Resume メトリクスなどの拡張余地

**完全実装後**:
- Akka/Pekko: ⭐⭐⭐⭐⭐ (5.0/5.0) - 完全同等
- 独自機能（柔軟な優先度、DropOldest）により、**一部で Akka/Pekko を超える**

### 8.3 最終推奨事項

1. **🔴 Stashing を Phase 2B〜3 で実装**（必須）
   - `Stashable` トレイトと `StashBuffer` の実装
   - ActorContext への統合

2. **🟡 Throughput 実装を Phase 1 で確認・完成**（推奨）
   - `throughput_hint()` の実装確認
   - MessageInvoker での yield ロジック確認

3. **🟡 リファクタリングプランで Suspend/Resume 拡張 (メトリクス) を明記**（推奨）
   - Phase 2 以降で `MailboxSuspensionMetrics` 相当を検討
   - Phase 2B に Stashing セクションを追加

4. **🟢 本レポートを `actor_scheduler_refactor.md` と統合**（推奨）

---

**レポート作成者**: Claude (Sonnet 4.5)
**作成日**: 2025-10-27
**参考文献**:
- [Akka Typed Mailboxes](https://doc.akka.io/docs/akka/current/typed/mailboxes.html)
- [Apache Pekko Mailboxes](https://pekko.apache.org/docs/pekko/current/typed/mailboxes.html)
- [protoactor-go mailbox.go](https://github.com/asynkron/protoactor-go/blob/dev/actor/mailbox.go)
