# ActorScheduler リファクタリングプラン レビュー

**レビュー実施日**: 2025-10-27
**レビュー対象**: `docs/design/actor_scheduler_refactor.md`
**レビュアー**: Claude (Sonnet 4.5)
**レビューモード**: `--ultrathink` (深層分析)

---

## エグゼクティブサマリー

### 総合評価

⭐⭐⭐⭐☆ **4.0 / 5.0**

| 評価項目 | スコア | コメント |
|---------|-------|---------|
| **アーキテクチャ設計** | ⭐⭐⭐⭐⭐ (5.0) | 責務分離が明確、参照実装との整合性が高い |
| **技術的実現可能性** | ⭐⭐⭐⭐☆ (4.5) | 実装可能だが、段階的移行の複雑さに注意 |
| **実装計画の具体性** | ⭐⭐⭐⭐⭐ (5.0) | フェーズ分割が明確、DoD が具体的 |
| **リスク管理** | ⭐⭐⭐⭐☆ (4.0) | ベンチマーク計画は優秀だが、実装ギャップに課題 |
| **ドキュメント品質** | ⭐⭐⭐⭐⭐ (5.0) | 詳細で構造化されている |

### 主要な強み

1. ✅ **明確な責務分離**: `ReadyQueueCoordinator` / `WorkerExecutor` / `MessageInvoker` の三層構造が SRP に準拠
2. ✅ **段階的移行戦略**: Phase 0〜4 の明確なロードマップと feature flag による並行運用
3. ✅ **包括的なベンチマーク計画**: レイテンシ/スループット/メモリの多面的評価
4. ✅ **参照実装との整合**: protoactor-go / Akka / Pekko との命名・概念の一致
5. ✅ **no_std 対応を考慮**: Shared 抽象と embedded ランタイムへの配慮

### 🔴 CRITICAL な発見事項

**Suspend/Resume 機能の実装ギャップ**

設計文書では Phase 0 の P0 課題として Suspend/Resume が挙げられていますが、**現在の実装では基礎的な機能が欠落**しています：

| 項目 | 設計文書での扱い | 実際の実装状況 | ギャップ |
|-----|----------------|---------------|---------|
| **型定義** | ✅ `InvokeResult::Suspended` 定義 | ✅ `SystemMessage::Suspend/Resume` 定義 | なし |
| **メールボックス実装** | ✅ 設計が明確（セクション 4.4.1） | ❌ **実装が欠落** | **重大** |
| **ActorCell 実装** | ✅ suspend 状態の評価を想定 | ❌ **特殊処理なし** | **重大** |
| **テストケース** | 📋 Phase 1 で 20 ケース計画 | ⚠️ 誤解を招くテストのみ | **重大** |

**詳細**: `docs/design/suspend_resume_status.md` を参照

**影響**:
- バックプレッシャ制御が不可能
- Stashing 機能が実装できない
- レート制限・初期化待機パターンが使えない
- **Akka/Pekko との互換性ギャップ**

**推奨**: Phase 0 で最小限の suspend/resume 実装を完成させることを **MUST タスク** として追加すべき

### 主要な懸念事項

1. ⚠️ **現状実装との乖離**: 多くの提案機能が「既に実装済み」または「実装が不完全」
2. ⚠️ **Phase 0 の前提条件が多い**: MUST タスクが複数のドキュメントに分散
3. ⚠️ **移行の複雑さ**: 既存 API との互換性維持が課題
4. ⚠️ **ロック粒度の最適化**: `spin::Mutex + VecDeque` のスケーラビリティ

---

## 1. アーキテクチャ設計の妥当性

### 1.1 責務分離の評価

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

設計文書では、現在の `ReadyQueueScheduler` / `ActorCell` が持つ複雑な責務を以下のように明確に分離：

```
現在:
┌─────────────────────────────────┐
│ ReadyQueueScheduler             │
│  - Ready queue 管理             │
│  - ワーカ駆動                   │
│  - Guardian 戦略                │
│  - エスカレーション配信          │
│  - Failure Telemetry            │
│  - Metrics Sink                 │
│  - receive timeout 共有         │
└─────────────────────────────────┘

設計後:
┌────────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│ReadyQueueCoordinator│   │ WorkerExecutor   │   │ MessageInvoker   │
│- Ready queue 調整   │   │- タスク生成      │   │- メッセージ実行  │
│- シグナル管理       │   │- ワーカ駆動      │   │- Suspend/Resume  │
│- 再登録制御         │   │- 再スケジュール  │   │- Guardian 連携   │
└────────────────────┘   └──────────────────┘   └──────────────────┘
```

**強み**:
- ✅ 各コンポーネントが**単一責任**に集中
- ✅ protoactor-go の `Dispatcher` / `Invoker` 概念との整合
- ✅ Akka の `Dispatcher` / `Mailbox` / `ExecutorService` モデルとの対応が明確

**参照実装との対応表**:

| cellex-rs (設計後) | protoactor-go | Akka/Pekko |
|-------------------|---------------|------------|
| ReadyQueueCoordinator | MailboxScheduler | Dispatcher (queue 管理部分) |
| WorkerExecutor | Dispatcher (実行部分) | ExecutorService |
| MessageInvoker | MessageInvoker | ActorCell.invoke() |

### 1.2 イベントフローの明確性

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 4.2 の Mermaid シーケンス図は以下の分岐を明示：
- ✅ 正常フロー（enqueue → dequeue → invoke → 完了）
- ✅ アクターサスペンド（Suspended → unregister）
- ✅ 異常終了（notify_failure → decide_restart → reschedule）
- ✅ メッセージ残存（ready_hint: true → requeue）

**特に優れている点**:
- バックプレッシャ判定とミドルウェアチェインの拡張ポイントを Phase 2B で明示する計画
- エッジケースをアーキテクチャレベルで把握可能

### 1.3 トレイト設計の評価

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 4.4 のトレイト定義が明確：

```rust
pub trait ReadyQueueCoordinator: Send + Sync {
    fn register_ready(&mut self, idx: MailboxIndex);
    fn unregister(&mut self, idx: MailboxIndex);
    fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut SmallVec<[MailboxIndex; 64]>);
    fn poll_wait_signal(&mut self, cx: &mut Context<'_>) -> Poll<()>;
    fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult);
    fn throughput_hint(&self) -> usize;
}
```

**強み**:
- ✅ メソッドシグネチャが明確で実装可能
- ✅ `SmallVec` による低アロケーション設計
- ✅ `poll_wait_signal` による非同期対応
- ✅ `handle_invoke_result` による責務の明確化

**既存実装との整合性**:

調査の結果、**このトレイトは既に実装済み**であることが判明：
- ファイル: `modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator/ready_queue_coordinator_trait.rs`
- 実装: `DefaultReadyQueueCoordinator`, `DefaultReadyQueueCoordinatorV2`, `LockFreeCoordinator`, `AdaptiveCoordinator`

**ギャップ**:
- ⚠️ 設計文書が「これから作る」トーンだが、実際には多くが実装済み
- 📋 文書を「既存実装のリファクタリング」として再構成すべき

---

## 2. 技術的実現可能性

### 2.1 実装スケルトンの充実度

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 4.4.1 と 4.7 でコードスケッチが豊富：

```rust
impl QueueMailbox {
    pub fn dequeue_batch(&self, max: usize) -> Vec<Envelope> {
        let mut batch = Vec::with_capacity(max);
        while batch.len() < max {
            if let Some(env) = self.system_queue.pop_front() {
                batch.push(env);
            } else {
                break;
            }
        }
        // ...
    }
}
```

**強み**:
- ✅ 実装の骨格が明確
- ✅ エラーハンドリングの方針が具体的（セクション 4.4.2）
- ✅ Guardian 連携の非同期チャネル設計が実用的

### 2.2 Shared 抽象との整合性

⭐⭐⭐⭐☆ **4.0 / 5.0** - 良好（注意点あり）

**評価理由**:

セクション 4.8 で Shared 抽象の使い分けを明示：

```rust
#[cfg(feature = "std")]
type MailboxConsumerShared<T> = ArcShared<T>;

#[cfg(not(feature = "std"))]
type MailboxConsumerShared<T> = RcShared<T>;
```

**強み**:
- ✅ `ArcShared` / `RcShared` / `StaticRefShared` の使い分けが明確
- ✅ `SharedDyn` によるトレイトオブジェクト化の方針

**懸念事項**:
- ⚠️ CLAUDE.md に「Shared 抽象にこだわりすぎなくてＯＫ」との記述
- ⚠️ `#[cfg(target_has_atomic = "ptr")]` の扱いが複雑
- 📋 実装時に過度な抽象化を避け、実用性を優先すべき

### 2.3 ランタイム抽象化の実現可能性

⭐⭐⭐⭐☆ **4.5 / 5.0** - 良好

**評価理由**:

セクション 4.7 の `WorkerExecutor` と `RuntimeShared` の設計：

```rust
pub trait RuntimeShared: Clone + Send + 'static {
    fn spawn(&self, task: impl Future<Output = ()> + Send + 'static);
    fn invoke(&self, idx: MailboxIndex) -> impl Future<Output = InvokeResult>;
    fn wait_with(&self, poll_fn: impl FnMut(&mut Context<'_>) -> Poll<()>) -> impl Future<Output = ()>;
}
```

**強み**:
- ✅ Tokio / Embassy / テストランタイムの統一抽象
- ✅ `poll_fn` による柔軟な待機機構

**懸念事項**:
- ⚠️ Embassy の `Spawner` は `Send` ではない可能性
- ⚠️ `impl Future` の返り値型が複雑化する可能性
- 📋 Phase 2A で実装時に調整が必要

---

## 3. 実装計画の評価

### 3.1 フェーズ分割の妥当性

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

Phase 0〜4 の分割が明確で段階的：

| Phase | 目標 | 期間（推定） |
|-------|------|------------|
| Phase 0 | 現状整理とベースライン取得 | 2 週間 |
| Phase 1 | ReadyQueueCoordinator 抽出 | 4 週間 |
| Phase 2A | WorkerExecutor 導入 | 3 週間 |
| Phase 2B | MessageInvoker 導入 | 4 週間 |
| Phase 3 | Registry / Observability 整備 | 3 週間 |
| Phase 4 | 統合・最適化 | 2 週間 |

**強み**:
- ✅ 各 Phase に明確な DoD（Definition of Done）
- ✅ ロールバック戦略が feature flag で担保
- ✅ ベンチマークによる回帰検知

### 3.2 Definition of Done の具体性

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 5.1 の DoD が非常に具体的：

**Phase 1 の例**:
- ✅ 単体テスト 20 ケース以上（正常系 8 / 異常系 7 / 境界値 5）
- ✅ カバレッジ 100%
- ✅ レイテンシ劣化 < 5%
- ✅ スループット ≥ 95%
- ✅ メモリオーバーヘッド < 10%
- ✅ 統合テスト 5 シナリオ（各 30 秒以内完了）

**強み**:
- ✅ 定量的な基準
- ✅ パフォーマンス回帰の防止
- ✅ テストカバレッジの担保

### 3.3 Phase 0 の前提条件の評価

⭐⭐⭐☆☆ **3.0 / 5.0** - 改善の余地あり

**評価理由**:

セクション 3.1 で Phase 0 前の MUST タスクが列挙されているが：

**懸念事項**:
- ⚠️ MUST タスクが 5 つのドキュメントに分散
  - `2025-10-12-actor-scheduler-options.md`
  - `2025-10-13-mailbox-runtime-status.md`
  - `2025-10-08-embedded-runtime-plan.md`
  - `2025-10-09-basic-feature-parity.md`
  - `2025-10-11-runtime-bundle-plan.md`
- ⚠️ これらの完了状況が不明
- ⚠️ **Suspend/Resume の実装が MUST タスクに含まれていない**

**推奨**:
- 📋 Phase 0 の MUST タスクを単一のトラッキング文書にまとめる
- 📋 各タスクの完了状況を明示する
- 🔴 **Suspend/Resume の基礎実装を MUST タスクに追加**

---

## 4. パフォーマンスとベンチマーク計画

### 4.1 ベンチマーク戦略

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 5.2 のベンチマーク計画が非常に包括的：

**計測指標**:
- ✅ レイテンシ（p50 / p95 / p99）
- ✅ スループット（messages/sec）
- ✅ CPU 使用率（`perf stat`）
- ✅ メモリ使用量（ヒープ使用量）

**許容値**:
- ✅ Phase 1: レイテンシ +5%, スループット 95%
- ✅ Phase 2: 累計 レイテンシ +10%, スループット 90%
- ✅ Phase 3: レイテンシ Phase 0 比 +5% へ回復

**自動化**:
- ✅ `.github/workflows/benchmarks.yml` で夜間実行
- ✅ `scripts/compare_benchmarks.py` で回帰検知
- ✅ 閾値超過時に Slack 通知

### 4.2 現在のベンチマーク結果

⭐⭐⭐⭐☆ **4.5 / 5.0** - 良好

**評価理由**:

セクション 5.3 で 2025-10-22 時点のベンチマーク結果を掲載：

**register_ready → drain_ready_cycle サイクル**:

| バッチサイズ | サイクル時間 | メッセージ単価 |
|------------|------------|---------------|
| 1 | 0.022 µs | 21.8 ns |
| 8 | 0.122 µs | 15.2 ns |
| 32 | 0.753 µs | 23.5 ns |
| 128 | 3.63 µs | 28.4 ns |

**懸念事項**:
- ⚠️ バッチサイズが大きくなるとメッセージ単価が上昇（21.8 ns → 42.9 ns）
- ⚠️ `BTreeSet` での重複排除がボトルネックの可能性
- 📋 lock-free バリアント（`RingQueue` バックエンド）の検証が重要

### 4.3 並行性ベンチマークの計画

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 5.1 と 5.2 で並行性ベンチマークを計画：

```bash
scripts/bench_concurrency.rs による 2/4/8/16 スレッド並行ベンチマーク
perf stat -e cycles,stalled-cycles-frontend,stalled-cycles-backend
```

**強み**:
- ✅ ロック待ち時間の計測
- ✅ 複数スレッド構成での評価
- ✅ `spin::Mutex` のスケーラビリティ検証

---

## 5. リスク管理の評価

### 5.1 ロールバック戦略

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 5.3 のロールバック戦略が明確：

```rust
#[cfg(feature = "new-scheduler")]
```

**強み**:
- ✅ Feature flag による新旧実装の並行運用
- ✅ Phase 4 完了時にデフォルト切り替え
- ✅ 1 週間のステージング観測
- ✅ ロールバック手順書（`scheduler_refactor_rollback.md`）の作成計画

### 5.2 エラーハンドリング方針

⭐⭐⭐⭐☆ **4.5 / 5.0** - 良好

**評価理由**:

セクション 4.4.2 でエラーハンドリングを明示：

**強み**:
- ✅ `InvokeResult::Failed { retry_after }` による再試行制御
- ✅ Guardian への委譲による最終判断
- ✅ 指数バックオフの適用

**懸念事項**:
- ⚠️ 致命的な mailbox 異常の扱いが Phase 2B 依存
- 📋 Phase 1 でも基本的なエラーパスを確保すべき

### 5.3 🔴 CRITICAL: Suspend/Resume 実装ギャップのリスク

⭐☆☆☆☆ **1.0 / 5.0** - 重大な問題

**評価理由**:

設計文書では Suspend/Resume が Phase 0 の P0 課題として挙げられているが、**現在の実装では基礎的な機能が欠落**：

**現状の問題**:

| 項目 | 期待される動作 | 実際の動作 | 影響 |
|-----|-------------|-----------|------|
| `SystemMessage::Suspend` | メールボックスがユーザーメッセージ処理を停止 | ❌ actor handler に渡されるだけ | **機能しない** |
| `SystemMessage::Resume` | メールボックスがユーザーメッセージ処理を再開 | ❌ actor handler に渡されるだけ | **機能しない** |
| suspend 状態管理 | ActorCell が suspend フラグを保持 | ❌ フラグが存在しない | **状態管理不可** |
| テストケース | suspend 後にユーザーメッセージが処理されないことを検証 | ⚠️ メッセージが handler に届くことのみ検証 | **誤解を招く** |

**詳細分析**: `docs/design/suspend_resume_status.md` を参照

**実装の欠落箇所**:

**ファイル**: `modules/actor-core/src/internal/actor/actor_cell.rs:241-292`

```rust
pub(super) fn dispatch_envelope(/* ... */) {
    // Stop の特殊処理は存在
    let should_stop = matches!(
        envelope.system_message(),
        Some(SystemMessage::Stop)
    );

    // Escalate の特殊処理も存在
    if let Some(SystemMessage::Escalate(failure)) = envelope.system_message().cloned() {
        // ...
    }

    // ❌ しかし Suspend/Resume の特殊処理は存在しない！
    // ↓ すべてのメッセージが handler に渡される
    let (message, priority) = envelope.into_parts();
    let handler_result = (self.handler)(&mut ctx, message);
}
```

**比較: protoactor-go の実装**:

```go
func (m *defaultMailbox) run() {
    for {
        // System メッセージを優先処理
        if msg = m.systemMailbox.Pop(); msg != nil {
            switch msg.(type) {
            case *SuspendMailbox:
                atomic.StoreInt32(&m.suspended, 1)  // ← suspend
            case *ResumeMailbox:
                atomic.StoreInt32(&m.suspended, 0)  // ← resume
            }
            continue
        }

        // ❗ suspend 中はユーザーメッセージをスキップ
        if atomic.LoadInt32(&m.suspended) == 1 {
            return  // ← ユーザーメッセージを処理しない
        }

        // ユーザーメッセージ処理
        if msg = m.userMailbox.Pop(); msg != nil {
            m.invoker.InvokeUserMessage(msg)
        }
    }
}
```

**旧実装（nexus-actor-rs）も完全実装を持っていた**:

```rust
struct MailboxSuspensionState {
    flag: AtomicBool,
    since: Mutex<Option<Instant>>,
    total_nanos: AtomicU64,
}

fn run(&self) {
    // suspend チェック
    if self.suspension.is_suspended() {
        return;  // ユーザーメッセージを処理しない
    }
}
```

**影響範囲**:

| 機能 | 影響 | 詳細 |
|-----|------|-----|
| **バックプレッシャ制御** | 🔴 不可能 | メールボックスを一時停止できない |
| **Stashing との連携** | 🔴 不可能 | suspend 中にメッセージを保留できない |
| **レート制限** | 🔴 不可能 | アクターを一時的に停止できない |
| **初期化待機** | 🔴 不可能 | 初期化完了まで処理を保留できない |
| **動的負荷調整** | 🔴 不可能 | 過負荷時にアクターを一時停止できない |

**Akka/Pekko との互換性**:

| 機能 | Akka/Pekko | cellex-rs | ギャップ |
|-----|-----------|-----------|---------|
| Suspend/Resume | ✅ 完全実装 | ❌ 未実装 | **大** |
| Stashing | ✅ 完全実装 | ❌ 未実装 | **大** |
| バックプレッシャ | ✅ Suspend 経由 | ❌ 未実装 | **大** |

**設計文書での扱い**:

設計文書では以下のように言及されているが、**現状実装の不完全性は明記されていない**：

**セクション 4.4**: `InvokeResult::Suspended` の定義
```rust
pub enum InvokeResult {
    Suspended { reason: SuspendReason, resume_on: ResumeCondition },
    // ...
}
```

**セクション 7 - オープン課題 P0**:
> Suspend/Resume の責務配置を Invoker 内に固定するかの判断 (Phase 0)

**セクション 10 Q5**:
> Suspend/Resume はどう伝播する？
> ActorCell が自身の状態を更新し、Invoker は `InvokeResult::Suspended` を返すことで...

**問題点**:
- ⚠️ 設計は明確だが、**現状実装が不完全であることが明記されていない**
- ⚠️ Phase 0 の P0 課題として挙げられているが、「設計判断」として扱われており、「基礎実装の完成」として認識されていない
- ⚠️ セクション 2.1 の実装ステータスで Suspend/Resume に言及がない

**推奨される対応**:

### Phase 0 で実施（🔴 最高優先度）:

1. **最小限の Suspend/Resume 実装**

```rust
pub struct ActorCell<MF, Strat> {
    // 既存フィールド
    suspended: AtomicBool,  // ← 追加
    suspend_since: Mutex<Option<Instant>>,  // ← 統計用
}

pub(super) fn dispatch_envelope(/* ... */) {
    // Suspend/Resume の特殊処理を追加
    match envelope.system_message() {
        Some(SystemMessage::Suspend) => {
            self.suspended.store(true, Ordering::SeqCst);
            // 統計記録
            return Ok(());  // handler には渡さない
        }
        Some(SystemMessage::Resume) => {
            self.suspended.store(false, Ordering::SeqCst);
            // 統計記録
            return Ok(());  // handler には渡さない
        }
        _ => {}
    }

    // suspend 中はユーザーメッセージをスキップ
    if self.suspended.load(Ordering::SeqCst) && envelope.system_message().is_none() {
        return Ok(());  // ユーザーメッセージを処理しない
    }

    // 通常処理
    // ...
}
```

2. **テストケース追加**

```rust
#[test]
fn test_suspend_blocks_user_messages() {
    // 1. ユーザーメッセージを送信
    actor_ref.send(42);

    // 2. Suspend を送信
    actor_ref.send_system(SystemMessage::Suspend);

    // 3. さらにユーザーメッセージを送信
    actor_ref.send(100);

    // 4. 処理を実行
    block_on(root.dispatch_next());
    block_on(root.dispatch_next());

    // 5. suspend 後のメッセージが処理されていないことを検証
    assert_eq!(*count.borrow(), 42, "Suspended actor should not process new messages");

    // 6. Resume を送信
    actor_ref.send_system(SystemMessage::Resume);
    block_on(root.dispatch_next());

    // 7. resume 後にメッセージが処理されることを検証
    block_on(root.dispatch_next());
    assert_eq!(*count.borrow(), 142, "Resumed actor should process pending messages");
}
```

3. **ADR 作成**
   - `docs/adr/2025-10-27-suspend-resume-implementation.md`
   - 設計判断と実装方針を文書化

### Phase 2B で実施:

4. **完全な Suspend/Resume 実装**
   - `InvokeResult::Suspended` との統合
   - ReadyQueueCoordinator での suspend 状態管理
   - Resume 条件（ExternalSignal / After / WhenCapacityAvailable）の実装

**ドキュメント訂正**:

5. **設計文書の修正**
   - セクション 2.1 に Suspend/Resume の実装ステータスを追加
   - セクション 3.1 の MUST タスクに Suspend/Resume 基礎実装を追加
   - セクション 7 の P0 課題の説明を修正（「設計判断」→「基礎実装完成」）

6. **比較文書の修正**
   - `mailbox_akka_pekko_comparison.md` の Suspend/Resume 評価を「❌ 未実装」に訂正
   - `mailbox_feature_comparison.md` の評価を更新

---

## 6. ドキュメント品質の評価

### 6.1 構造と可読性

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

- ✅ セクション構成が論理的（背景 → 現状 → 課題 → 目標 → ロードマップ → FAQ）
- ✅ Mermaid 図による視覚化
- ✅ コードスケッチによる具体性
- ✅ 表形式での比較が分かりやすい

### 6.2 技術的正確性

⭐⭐⭐☆☆ **3.5 / 5.0** - 改善の余地あり

**評価理由**:

**強み**:
- ✅ 参照実装（protoactor-go / Akka）との対応が正確
- ✅ ベンチマーク結果が具体的

**懸念事項**:
- ⚠️ 既存実装との整合性が不明瞭（多くが「これから作る」トーンだが実際には実装済み）
- ⚠️ **Suspend/Resume の実装ステータスが誤解を招く**
- 📋 実装ステータスの明示的な記載が必要

### 6.3 実装ガイダンスの充実度

⭐⭐⭐⭐⭐ **5.0 / 5.0** - 優秀

**評価理由**:

セクション 10 の FAQ が非常に有用：

- ✅ Q1: MailboxIndex からの Mailbox 取得方法
- ✅ Q2: Coordinator と Executor の役割分担
- ✅ Q3: 並行アクセスの排他制御
- ✅ Q4: ベンチマークの運用方法
- ✅ Q5: Suspend/Resume の伝播経路

**推奨**:
- 📋 実装時の注意点（セクション 4.11）を FAQ に統合すると良い

---

## 7. 改善提案

### 7.1 🔴 即座に実施すべき改善

#### 1. Suspend/Resume 実装の完成（CRITICAL）

**優先度**: 🔴 P0 - 最高

**理由**: 現在の実装では基礎的な機能が欠落しており、多くの高度な機能が実現不可能

**アクション**:
- Phase 0 の MUST タスクに追加
- 最小限の実装を 1 週間以内に完成
- テストケース 10 件追加
- ADR で設計を文書化

#### 2. 実装ステータスの明示

**優先度**: 🔴 P0 - 最高

**理由**: 「これから作る」のか「既に存在する」のかが不明瞭

**アクション**:
- セクション 2.1 に各コンポーネントの実装ステータスを追加：
  - ✅ 完全実装
  - 🚧 部分実装（具体的な欠落を明記）
  - ⏳ 未実装

**例**:

| コンポーネント | ステータス | 詳細 |
|-------------|---------|------|
| ReadyQueueCoordinator trait | ✅ 完全実装 | v1/v2 実装が存在 |
| InvokeResult / SuspendReason | ✅ 完全実装 | API 定義済み |
| Suspend/Resume 処理 | ❌ **未実装** | ActorCell に実装なし |
| WorkerExecutor | ⏳ 未実装 | Phase 2A で実装予定 |
| MessageInvoker | ⏳ 未実装 | Phase 2B で実装予定 |

#### 3. Phase 0 MUST タスクの統合

**優先度**: 🟡 P1 - 高

**理由**: MUST タスクが複数ドキュメントに分散

**アクション**:
- `docs/design/phase0_must_tasks.md` を作成
- 各タスクのトラッキング ID と完了状況を記載
- セクション 3.1 から参照

### 7.2 🟡 Phase 0 で実施すべき改善

#### 4. 責務マッピング図の作成

**優先度**: 🟡 P1 - 高

**理由**: セクション 8 の次アクション Week 1 に記載されているが重要

**アクション**:
- `docs/design/scheduler_component_mapping.puml` を作成
- 現在 → Phase 1 → Phase 2 → Phase 4 の変遷を視覚化

#### 5. ベースラインベンチマークの取得

**優先度**: 🟡 P1 - 高

**理由**: 回帰検知の基準が必要

**アクション**:
- `benchmarks/baseline_phase0.txt` を生成
- `scripts/compare_benchmarks.py` を実装
- CI に統合

### 7.3 🟢 将来的な改善

#### 6. lock-free バリアントの検証

**優先度**: 🟢 P2 - 中

**理由**: `spin::Mutex + BTreeSet` のスケーラビリティ懸念

**アクション**:
- `RingQueue` バックエンドの実装
- 並行性ベンチマークでの比較
- ロック待ち時間の計測

#### 7. 命名の最終決定

**優先度**: 🟢 P3 - 低

**理由**: Phase 4 で `ActorSchedulerFrontend` への改名検討

**アクション**:
- Phase 4 で命名 ADR を作成
- コードとドキュメントを同時更新

---

## 8. 推奨アクション（優先度順）

### 🔴 即座に実施（Phase 0 開始前）

1. **Suspend/Resume の最小限実装** (1 週間)
   - ActorCell に `suspended: AtomicBool` を追加
   - `dispatch_envelope` で特殊処理
   - テストケース 10 件追加

2. **実装ステータスの明示** (1 日)
   - セクション 2.1 に実装ステータス表を追加
   - 各コンポーネントの状況を明記

3. **Suspend/Resume 関連ドキュメントの訂正** (半日)
   - `mailbox_akka_pekko_comparison.md` を修正
   - 本レビュー文書を参考文献に追加

### 🟡 Phase 0 で実施（2 週間）

4. **Phase 0 MUST タスクの統合** (2 日)
   - `phase0_must_tasks.md` を作成
   - トラッキング表を整備

5. **責務マッピング図の作成** (3 日)
   - `scheduler_component_mapping.puml` を作成
   - 変遷を視覚化

6. **ベースラインベンチマークの取得** (3 日)
   - `baseline_phase0.txt` を生成
   - `compare_benchmarks.py` を実装

7. **Suspend/Resume ADR の作成** (2 日)
   - `2025-10-27-suspend-resume-implementation.md`
   - 設計判断を文書化

### 🟢 Phase 1 以降で実施

8. **lock-free バリアントの検証** (Phase 1)
   - `RingQueue` バックエンド実装
   - 並行性ベンチマーク

9. **完全な Suspend/Resume 実装** (Phase 2B)
   - `InvokeResult::Suspended` との統合
   - Resume 条件の実装

10. **命名の最終決定** (Phase 4)
    - 命名 ADR を作成
    - 一括リネーム

---

## 9. 結論

### 9.1 総合評価

⭐⭐⭐⭐☆ **4.0 / 5.0** - 優秀な設計だが実装ギャップに注意

**強み**:
- ✅ アーキテクチャ設計が明確で SRP に準拠
- ✅ フェーズ分割とロールバック戦略が現実的
- ✅ ベンチマーク計画が包括的
- ✅ 参照実装との整合性が高い
- ✅ ドキュメントが詳細で実装可能

**懸念事項**:
- 🔴 **Suspend/Resume の実装ギャップが重大**
- ⚠️ 現状実装との整合性が不明瞭
- ⚠️ Phase 0 の前提条件が多い
- ⚠️ lock-free 化の難易度が不明

### 9.2 リファクタリング実施の推奨

**推奨**: ✅ **実施すべき**（ただし Suspend/Resume の完成を前提条件に追加）

**理由**:
1. 現在の `ReadyQueueScheduler` / `ActorCell` の責務が過度に複雑
2. 段階的移行戦略が現実的
3. ロールバック機構が整備されている
4. ベンチマークによる品質担保が可能

**条件**:
- 🔴 **Phase 0 で Suspend/Resume の最小限実装を完成させること**
- 🟡 Phase 0 の MUST タスクを統合・トラッキングすること
- 🟡 実装ステータスを明示すること

### 9.3 リスク評価

| リスク項目 | 確率 | 影響 | 緩和策 |
|-----------|------|------|-------|
| Suspend/Resume 実装の複雑化 | 中 | 🔴 高 | Phase 0 で最小限実装、Phase 2B で拡張 |
| パフォーマンス劣化 | 中 | 🔴 高 | 各 Phase でベンチマーク、feature flag でロールバック |
| Phase 0 MUST タスクの遅延 | 高 | 🟡 中 | タスク統合、定期レビュー |
| ランタイム抽象の複雑化 | 中 | 🟡 中 | Phase 2A で実装検証 |
| lock-free 化の難航 | 低 | 🟢 低 | 既存 Mutex 実装を Phase 1 で継続利用 |

### 9.4 成功の鍵

1. **Suspend/Resume の早期完成**: Phase 0 で最小限実装を必ず完成させる
2. **段階的移行の徹底**: feature flag を活用し、各 Phase で確実にロールバック可能にする
3. **ベンチマークの自動化**: CI で継続的に回帰検知
4. **実装ステータスの可視化**: 何が実装済みで何が未実装かを明確化
5. **ADR による設計判断の記録**: 後から振り返れるようにする

---

## 10. 参照文献

- `docs/design/actor_scheduler_refactor.md` - 本レビューの対象文書
- `docs/design/suspend_resume_status.md` - Suspend/Resume 実装ギャップの詳細分析
- `docs/design/mailbox_akka_pekko_comparison.md` - Akka/Pekko との機能比較
- `docs/design/mailbox_feature_comparison.md` - protoactor-go との機能比較
- `docs/sources/nexus-actor-rs/modules/actor-std/src/actor/dispatch/mailbox/` - 旧実装の参照
- `docs/sources/protoactor-go/actor/mailbox.go` - protoactor-go の参照実装
- `modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator/` - 既存実装
- `modules/actor-core/src/internal/actor/actor_cell.rs` - ActorCell の現在実装

---

**レビュー完了日**: 2025-10-27
**次回レビュー推奨**: Phase 0 完了時（Suspend/Resume 実装後）
