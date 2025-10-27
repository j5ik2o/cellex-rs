# メールボックス機能比較レポート

**作成日**: 2025-10-27
**対象**: cellex-rs メールボックス実装
**比較対象**: protoactor-go、nexus-actor-rs

---

## エグゼクティブサマリー

cellex-rs のメールボックス実装は、参照実装（protoactor-go、nexus-actor-rs）の**主要機能を実装済み**です。2025-10-27 時点で ActorCell レベルの Suspend/Resume 制御が導入され、ユーザーメッセージの停止と Resume 後の再開が確認できました。一部の高度な機能（ミドルウェアチェイン、スループット制御、サスペンション統計）は**部分的実装または未実装**ですが、`actor_scheduler_refactor.md` のリファクタリングプランで**これらの機能が考慮されている**ことを確認しました。

**結論**: ✅ 基本機能は実装済み（Suspend/Resume 含む）。リファクタリングプランで不足機能の追加とメトリクス拡充が計画済み。

**詳細**: Suspend/Resume 実装の背景と今後の改善項目は `mailbox_suspend_resume_plan.md` を参照。

---

## 1. 参照実装の主要機能

### 1.1 protoactor-go のメールボックス機能

| 機能 | 説明 | 実装箇所 |
|-----|------|---------|
| **User/System メッセージ分離** | 優先度別キュー（system > user） | `userMailbox`, `systemMailbox` |
| **MessageInvoker** | メッセージ処理の抽象化 | `InvokeUserMessage()`, `InvokeSystemMessage()` |
| **Dispatcher 連携** | スケジューリングとスループット制御 | `dispatcher.Schedule()`, `Throughput()` |
| **MailboxMiddleware** | メッセージインターセプト | `MailboxStarted()`, `MessagePosted()`, `MessageReceived()`, `MailboxEmpty()` |
| **Suspend/Resume** | メッセージ処理の一時停止・再開 | `SuspendMailbox`, `ResumeMailbox` |
| **スケジューリング制御** | idle/running 状態管理 | `schedulerStatus` (atomic CAS) |
| **スループット制限** | 公平性のためのメッセージ処理数制限 | `i, t := 0, m.dispatcher.Throughput()` |
| **エスカレーション** | パニック時のエラー伝播 | `invoker.EscalateFailure()` |

**重要な設計パターン**:
```go
// スケジューリング: idle → running の CAS 操作
if atomic.CompareAndSwapInt32(&m.schedulerStatus, idle, running) {
    m.dispatcher.Schedule(m.processMessages)
}

// スループット制御: 公平性のための yield
i, t := 0, m.dispatcher.Throughput()
for {
    if i > t {
        i = 0
        runtime.Gosched()  // 他の goroutine に CPU を譲る
    }
    i++
    // メッセージ処理...
}
```

### 1.2 nexus-actor-rs のメールボックス機能

| 機能 | 説明 | 実装箇所 |
|-----|------|---------|
| **User/System メッセージ分離** | 優先度別キュー | `default_mailbox.rs` |
| **MessageInvoker** | Rust 版の抽象化 | `MessageInvokerHandle` |
| **Dispatcher 連携** | スケジューリング | `DispatcherHandle` |
| **MailboxMiddleware** | ミドルウェアチェイン | `MailboxMiddlewareHandle` |
| **Suspend/Resume** | 状態管理 | `MailboxSuspensionState` |
| **メトリクス収集** | レイテンシとサスペンション統計 | `QueueLatencyTracker`, `MailboxSuspensionMetrics` |
| **キューレイテンシ計測** | ヒストグラム化 | `LatencyHistogram`, 17バケット |
| **バックログ感度** | キュー長に応じた処理調整 | `backlog_sensitivity` |

**重要な拡張機能**:
```rust
// レイテンシヒストグラム（17バケット: 1µs 〜 500ms）
const LATENCY_BUCKET_BOUNDS: [u64; 17] = [
    1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000,
    1_000_000, 2_500_000, 5_000_000, 10_000_000, 25_000_000,
    50_000_000, 100_000_000, 250_000_000, 500_000_000,
];

// サスペンション統計
pub struct MailboxSuspensionMetrics {
    pub resume_events: u64,
    pub total_duration: Duration,
}
```

---

## 2. cellex-rs の現状実装

### 2.1 実装済み機能

| 機能 | 実装状況 | 実装箇所 | 参照実装との比較 |
|-----|---------|---------|-----------------|
| **User/System メッセージ分離** | ✅ 完全実装 | `PriorityEnvelope`, `SystemMessage` | protoactor-go と同等 |
| **優先度付きキュー** | ✅ 完全実装 | `priority: i8` フィールド | nexus-actor-rs より柔軟（8段階 vs 2段階） |
| **Mailbox トレイト** | ✅ 完全実装 | `modules/actor-core/src/api/mailbox.rs` | より汎用的な抽象化 |
| **QueueMailbox** | ✅ 完全実装 | `QueueMailbox<Q, S>` | ジェネリックなキュー・シグナル抽象 |
| **MailboxProducer** | ✅ 完全実装 | `QueueMailboxProducer` | protoactor-go の `PostUserMessage` 相当 |
| **MailboxConsumer** | ✅ 完全実装 | `MailboxConsumer` trait | nexus-actor-rs の `MessageInvoker` 相当 |
| **Suspend/Resume** | ✅ 実装済み | `ActorCell`, `SystemMessage::Suspend/Resume` | ユーザーメッセージの停止/再開を `ActorCell` で制御 |
| **メトリクスシンク** | ✅ 完全実装 | `set_metrics_sink()` | nexus-actor-rs より簡潔 |
| **スケジューラフック** | ✅ 完全実装 | `set_scheduler_hook()` | ReadyQueue 連携 |
| **非同期受信** | ✅ 完全実装 | `recv()` → `Future<Output = Result<M, QueueError>>` | async/await 対応 |

### 2.2 部分的実装または未実装の機能

| 機能 | 実装状況 | 参照実装での実装 | cellex-rs での対応 |
|-----|---------|-----------------|-------------------|
| **MailboxMiddleware** | ⚠️ 未実装 | protoactor-go: 4つのフック | リファクタリングプランで計画済み（Phase 2B） |
| **スループット制限** | ⚠️ 部分的 | protoactor-go: `Throughput()` | `throughput_hint()` がトレイトに存在（実装は未確認） |
| **レイテンシヒストグラム** | ❌ 未実装 | nexus-actor-rs: 17バケット | メトリクスシンク経由で別途実装の可能性 |
| **サスペンション統計** | ❌ 未実装 | nexus-actor-rs: `MailboxSuspensionMetrics` | リファクタリングプランで言及なし |
| **バックログ感度** | ❌ 未実装 | nexus-actor-rs: `backlog_sensitivity` | リファクタリングプランで言及なし |

### 2.3 cellex-rs の独自拡張

| 機能 | 説明 | 利点 |
|-----|------|-----|
| **ジェネリックなキュー抽象** | `MailboxQueue<M>` trait | 任意のキュー実装を差し替え可能 |
| **シグナル抽象** | `MailboxSignal` trait | Tokio/Embassy/テスト環境で統一 API |
| **ReadyQueue 統合** | `set_scheduler_hook()` | Coordinator への自動通知 |
| **型安全なエラー** | `MailboxError<M>` | メッセージ型を保持したエラー |
| **柔軟な優先度** | `i8` 型（-128 〜 127） | システムメッセージ優先度の細かい制御 |

---

## 3. リファクタリングプランでの扱い

### 3.1 明示的に計画されている機能

#### ✅ MailboxMiddleware（Phase 2B）

**ドキュメント箇所**: セクション 4.4 「トレイトとインタフェース素案」

```rust
/// MessageInvoker 実装に前後処理を提供するミドルウェアチェイン
pub trait MiddlewareChain {
    fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()>;
    fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult);
}
```

**対応する protoactor-go の機能**:
```go
type MailboxMiddleware interface {
    MailboxStarted()
    MessagePosted(message interface{})
    MessageReceived(message interface{})
    MailboxEmpty()
}
```

**cellex-rs の実装方針**:
- `MessageInvoker` に `MiddlewareChain` を注入
- `before_invoke` で `ControlFlow::Break` を返すことで処理を保留
- テレメトリやロギングはここで集約

#### ✅ スループット制御（Phase 1〜3）

**ドキュメント箇所**: セクション 4.4、4.7

```rust
pub trait ReadyQueueCoordinator: Send + Sync {
    /// throughput（Akka の dispatcher-throughput 相当）のヒント値を返す
    fn throughput_hint(&self) -> usize;
}
```

**実装方針**（セクション 4.4.1）:
> 処理ループは `throughput_hint` を参照し、指定件数に達したら自発的に `InvokeResult::Yielded` を返すことで公平性を担保する。

**対応する protoactor-go の実装**:
```go
i, t := 0, m.dispatcher.Throughput()
for {
    if i > t {
        i = 0
        runtime.Gosched()
    }
    i++
    // メッセージ処理...
}
```

#### ✅ Observability Hub（Phase 3）

**ドキュメント箇所**: セクション 4.1、4.10

> **Observability Hub**: Failure Telemetry / Metrics Sink / トレース送出を統一的に収集し、enqueue/dequeue の計測ポイントを整理する。

**メトリクス最低ライン**（セクション 4.10）:
- `actor.mailbox.enqueued_total{actor,mailbox}`
- `actor.mailbox.depth{actor}`
- `scheduler.ready_queue.depth`
- `scheduler.worker.busy_ratio{worker}`
- `scheduler.invoke.duration_ms{actor}`
- `scheduler.latency_ms{actor}`
- `dead_letters_total{reason}`

### 3.2 言及されていない機能

| 機能 | 参照実装での実装 | リファクタリングプランでの扱い |
|-----|-----------------|----------------------------|
| **レイテンシヒストグラム** | nexus-actor-rs: 17バケット | ❌ 明示的な言及なし（Observability Hub で暗黙的にカバーの可能性） |
| **サスペンション統計** | nexus-actor-rs: `MailboxSuspensionMetrics` | ❌ 明示的な言及なし |
| **バックログ感度** | nexus-actor-rs: `backlog_sensitivity` | ❌ 明示的な言及なし |

---

## 4. 機能カバレッジ評価

### 4.1 カバレッジマトリクス

| 機能カテゴリ | protoactor-go | nexus-actor-rs | cellex-rs 現状 | リファクタ後 |
|------------|--------------|---------------|---------------|-------------|
| **基本メッセージング** | ✅ | ✅ | ✅ | ✅ |
| User/System 分離 | ✅ | ✅ | ✅ | ✅ |
| 優先度制御 | ✅ (2段階) | ✅ (2段階) | ✅ (8段階) | ✅ (8段階) |
| **スケジューリング** | ✅ | ✅ | ✅ | ✅ |
| Dispatcher 連携 | ✅ | ✅ | ✅ | ✅ |
| スループット制限 | ✅ | ✅ | ⚠️ | ✅ |
| **状態管理** | ✅ | ✅ | ✅ | ✅ |
| Suspend/Resume | ✅ | ✅ | ✅ | ✅ |
| idle/running 制御 | ✅ | ✅ | ✅ | ✅ |
| **拡張機能** | ⚠️ | ✅ | ⚠️ | ✅ |
| Middleware | ✅ | ✅ | ❌ | ✅ |
| メトリクス収集 | ❌ | ✅ | ⚠️ | ✅ |
| レイテンシ統計 | ❌ | ✅ | ❌ | ❓ |
| サスペンション統計 | ❌ | ✅ | ❌ | ❓ |

**凡例**: ✅ 完全実装 / ⚠️ 部分実装 / ❌ 未実装 / ❓ 明示的な計画なし

### 4.2 総合評価

#### 基本機能（メッセージング、スケジューリング、状態管理）

**評価**: ⭐⭐⭐⭐⭐ (5.0/5.0)

cellex-rs は参照実装の基本機能を**完全に実装**しており、以下の点で優れています：
- より柔軟な優先度制御（8段階 vs 2段階）
- ジェネリックなキュー・シグナル抽象化
- async/await ネイティブな API

#### 拡張機能（Middleware、メトリクス、統計）

**評価**: ⭐⭐⭐☆☆ (3.0/5.0)

- **Middleware**: リファクタリングプラン（Phase 2B）で実装予定 ✅
- **メトリクス収集**: Observability Hub（Phase 3）で統合予定 ✅
- **レイテンシ統計**: 明示的な計画なし ⚠️
- **サスペンション統計**: 明示的な計画なし ⚠️

---

## 5. 推奨事項

### 5.1 即座に実施すべき改善（優先度: 高）

#### 推奨 1: レイテンシヒストグラムの実装を Phase 3 に追加

**問題**: nexus-actor-rs の有用な機能がリファクタリングプランで考慮されていない。

**解決策**: Observability Hub（Phase 3）の設計にレイテンシヒストグラムを明示的に追加：

```rust
pub struct MailboxMetrics {
    enqueued_total: AtomicU64,
    dequeued_total: AtomicU64,
    depth: AtomicUsize,
    latency_histogram: Arc<LatencyHistogram>,  // ← 追加
}
```

**nexus-actor-rs のバケット設計を参考**:
- 17バケット: 1µs, 5µs, 10µs, 25µs, 50µs, 100µs, 250µs, 500µs, 1ms, 2.5ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms
- パーセンタイル計算（p50, p95, p99）のサポート

#### 推奨 2: サスペンション統計の実装を検討

**問題**: Suspend/Resume の頻度や継続時間を可視化する仕組みがない。

**解決策**: Phase 3 で以下の統計を追加：

```rust
pub struct SuspensionMetrics {
    pub total_suspensions: u64,
    pub total_suspended_duration: Duration,
    pub average_suspended_duration: Duration,
}
```

### 5.2 中期的に検討すべき改善（優先度: 中）

#### 推奨 3: バックログ感度の実装

nexus-actor-rs の `backlog_sensitivity` 機能を Phase 2B〜3 で検討：
- キュー滞留が一定数を超えたら優先度を動的に調整
- バックプレッシャ制御との統合

#### 推奨 4: MailboxMiddleware の実装優先度を明確化

Phase 2B で実装予定だが、具体的なユースケースを明示すべき：
- テレメトリ収集（Observability Hub との連携）
- デバッグロギング
- レート制限（バックプレッシャ）
- メッセージ検証

### 5.3 ドキュメント改善（優先度: 中）

#### 推奨 5: 機能カバレッジ表を `actor_scheduler_refactor.md` に追加

**追加箇所**: セクション 2「現状整理」または セクション 4.1「コンポーネント構成」

本レポートの「4.1 カバレッジマトリクス」を元に、参照実装との機能比較表を追加すべき。これにより、リファクタリングの範囲と目標が明確になります。

---

## 6. 結論

### 6.1 主要な発見

1. **cellex-rs の基本メールボックス機能は参照実装と同等レベル**
   - User/System 分離、優先度制御、Dispatcher 連携は完全実装
   - ✅ Suspend/Resume が ActorCell レベルで実装され、ユーザーメッセージの停止と再開が確認済み
   - ジェネリックな抽象化により、参照実装より柔軟な設計

2. **リファクタリングプランは不足機能を適切にカバー**
   - MailboxMiddleware（Phase 2B）
   - スループット制限（Phase 1〜3）
   - Observability Hub（Phase 3）

3. **nexus-actor-rs の高度な統計機能は部分的に未計画**
   - レイテンシヒストグラム: Observability Hub で暗黙的にカバーされる可能性
   - サスペンション統計: 明示的な計画なし
   - バックログ感度: 明示的な計画なし

### 6.2 総合評価

**現状実装**: ⭐⭐⭐⭐☆ (4.0/5.0)
- 基本機能（Suspend/Resume を含む）は実装済み
- 拡張機能（Middleware、統計）は引き続き課題

**リファクタリング後**: ⭐⭐⭐⭐⭐ (5.0/5.0 想定)
- Middleware と Observability Hub の追加で参照実装を超える可能性

### 6.3 最終的な回答

> **質問**: 参照実装と同等のメールボックス機能が現状の実装に盛り込まれているか？
>
> **回答**: ✅ **はい、基本機能は同等水準です。**
>
> - 基本メッセージング、優先度制御、Dispatcher 連携は**完全実装**
> - Suspend/Resume が ActorCell レベルで動作（詳細: `mailbox_suspend_resume_plan.md`）
> - Middleware、スループット制限などの拡張機能は**部分的実装**
>
> **質問**: このリファクタリングプランでも同等の機能が想定されているか？
>
> **回答**: ✅ **はい、想定されています。**
>
> - MailboxMiddleware（Phase 2B）
> - スループット制限（`throughput_hint`）
> - Observability Hub（Phase 3）
>
> **ただし**、以下の機能は明示的な計画がありません：
> - レイテンシヒストグラム（17バケット）
> - サスペンション統計
> - バックログ感度
>
> これらの追加を推奨します。

---

**レポート作成者**: Claude (Sonnet 4.5)
**作成日**: 2025-10-27
**次回レビュー推奨時期**: Phase 2B 完了後（MailboxMiddleware 実装後）
