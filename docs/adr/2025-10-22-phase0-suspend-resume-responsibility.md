# ADR-002: Suspend/Resume 責務配置

## ステータス

提案中

## コンテキスト

ActorScheduler リファクタリングにおいて、アクターの Suspend（一時停止）と Resume（再開）の責務をどのコンポーネントに配置するかが設計上の重要な決定事項となっています。

### 現在の問題点

**現行実装（Phase 0 以前）**:
- `ActorCell` が Suspend 状態の判定とメッセージ処理の両方を担当
- `ReadyQueueContext` は Suspend 状態を直接認識せず、単に mailbox を再登録
- Suspend/Resume のシグナル伝播経路が不明瞭

### 検討すべき論点

1. **Suspend 状態の判定場所**: どのコンポーネントが「アクターをサスペンドすべきか」を判断するか？
2. **Resume 契機の管理**: 外部シグナル、タイムアウト、容量回復などの Resume 条件をどう扱うか？
3. **Ready Queue との連携**: Suspend されたアクターを ready queue からどう除外し、Resume 時にどう再登録するか？
4. **状態の一貫性**: Suspend 状態が複数コンポーネント間で矛盾しないことをどう保証するか？

### 制約条件

- ReadyQueueCoordinator は状態を持たない（ステートレス）方針
- MessageInvoker は ActorCell の状態を参照できる
- Phase 2B で実装予定（Phase 0 では方針のみ確定）

### 前提条件

- `InvokeResult` 列挙型に `Suspended` バリアントが存在（Phase 0 で実装済み）
- `ReadyQueueCoordinator::handle_invoke_result` が結果に応じて再登録を制御（Phase 0 で実装済み）

## 決定

### 選択した解決策

**Suspend/Resume の責務は MessageInvoker と ReadyQueueCoordinator で分担する。**

#### 責務分担

| コンポーネント | Suspend 時の責務 | Resume 時の責務 |
|----------------|------------------|------------------|
| **MessageInvoker** | - ActorCell の状態を評価<br>- Suspend 理由を判定<br>- `InvokeResult::Suspended` を返却 | - （なし：ActorCell が直接 Coordinator へ通知） |
| **ReadyQueueCoordinator** | - `InvokeResult::Suspended` を受信<br>- `unregister(idx)` を実行<br>- ready queue から除外 | - `register_ready(idx)` を受信<br>- ready queue へ再登録 |
| **ActorCell** | - 自身の状態を `Suspended` に設定<br>- Suspend 理由と Resume 条件を保持 | - 自身の状態を `Running` に更新<br>- `MailboxRegistry` 経由で Coordinator へ通知 |
| **MailboxRegistry** | - （なし） | - ActorCell からの Resume 通知を仲介<br>- Coordinator の `register_ready` を呼び出し |

#### シーケンス

**Suspend フロー**:
```
MessageInvoker
  ↓ actor_state() で状態確認
ActorCell (state: Running)
  ↓ サスペンド条件を検出
ActorCell (state: Suspended)
  ↓ InvokeResult::Suspended { reason, resume_on } を返却
MessageInvoker
  ↓ handle_invoke_result(idx, Suspended)
ReadyQueueCoordinator
  ↓ unregister(idx)
Ready Queue (idx を除外)
```

**Resume フロー**:
```
External Event / Timeout / Capacity Available
  ↓ resume 契機
ActorCell (state: Suspended → Running)
  ↓ registry.notify_resume(idx)
MailboxRegistry
  ↓ coordinator.register_ready(idx)
ReadyQueueCoordinator
  ↓ register_ready(idx)
Ready Queue (idx を再登録)
  ↓ WorkerExecutor が処理
MessageInvoker (再度メッセージ処理開始)
```

#### 設計原則

1. **単一責任原則**:
   - Invoker は「判定」のみ
   - Coordinator は「登録/除外」のみ
   - ActorCell は「状態管理」のみ

2. **ステートレス Coordinator**:
   - Coordinator は Suspend 状態を保持しない
   - 全ての状態は ActorCell が所有

3. **非同期 Resume**:
   - Resume 契機は ActorCell が管理（タイマー、シグナル、容量監視）
   - Registry 経由で Coordinator へ通知

### 代替案

#### 代替案 1: Coordinator が Suspend 状態を管理

- **概要**: ReadyQueueCoordinator 内部に `suspended: HashSet<MailboxIndex>` を保持し、Resume 時に Coordinator が判定
- **利点**:
  - Resume 時の判定ロジックが集中
  - ActorCell から Registry への通知が不要
- **欠点**:
  - Coordinator がステートフルになり、Phase 1 の設計方針と矛盾
  - Suspend 理由と Resume 条件を Coordinator が保持する必要があり、複雑化
  - ActorCell の状態と Coordinator の状態が二重管理となり不整合リスク
- **不採用の理由**: ステートレス Coordinator の原則に反する

#### 代替案 2: Invoker が Resume も制御

- **概要**: MessageInvoker が Resume 契機も監視し、Coordinator への通知を全て担当
- **利点**:
  - Coordinator への通知経路が一本化
  - ActorCell と Coordinator の疎結合
- **欠点**:
  - Invoker が Resume 契機（外部シグナル、タイムアウト等）を監視する責務を持つことになり、肥大化
  - Resume 契機は ActorCell の状態に依存するため、Invoker が ActorCell の内部状態を深く知る必要がある
  - 複数の Resume 条件（タイムアウト、シグナル、容量回復）を Invoker が扱うのは責務過多
- **不採用の理由**: Invoker の責務が肥大化し、単一責任原則に反する

#### 代替案 3: Suspend 専用コンポーネント（SuspendManager）

- **概要**: Suspend/Resume 専用の `SuspendManager` を新設し、全ての状態管理と Resume 契機監視を担当
- **利点**:
  - Suspend/Resume のロジックが完全に分離
  - 複雑な Resume 条件（タイムアウト、複合条件等）に対応しやすい
- **欠点**:
  - 新しいコンポーネントが増え、アーキテクチャが複雑化
  - ActorCell、Invoker、Coordinator との連携が増え、シグナル伝播経路が複雑
  - Phase 0-4 のロードマップに SuspendManager が含まれておらず、後付けとなる
- **不採用の理由**: Phase 2B の範囲を超え、将来的な拡張課題として Phase 4 以降で検討すべき

## 結果

### 利点

1. **責務の明確化**:
   - Invoker: 判定
   - Coordinator: 登録/除外
   - ActorCell: 状態管理
   - 各コンポーネントの責務が明確で、単一責任原則を遵守

2. **ステートレス Coordinator の維持**:
   - Coordinator は Suspend 状態を保持せず、`InvokeResult` に基づいて動作
   - Phase 1 の設計方針と整合

3. **拡張性**:
   - Resume 条件（タイムアウト、外部シグナル、容量回復）は ActorCell 内で完結
   - 新しい Resume 条件追加時も Coordinator/Invoker の変更不要

4. **テスト容易性**:
   - Invoker のテスト: ActorCell の状態をモックして `InvokeResult::Suspended` が返ることを確認
   - Coordinator のテスト: `InvokeResult::Suspended` を渡して `unregister` が呼ばれることを確認
   - ActorCell のテスト: Resume 契機で Coordinator に通知することを確認

### 欠点・トレードオフ

1. **シグナル伝播経路の複雑性**:
   - Resume 時に ActorCell → Registry → Coordinator という経路が必要
   - ただし、これは既存の spawn フローと同じ経路なので、新しい複雑性ではない

2. **Resume 時のレース条件リスク**:
   - Resume 通知と新しいメッセージ enqueue が同時発生した場合、重複登録の可能性
   - **対策**: Coordinator の `register_ready` は既に重複登録を防止（Phase 0 実装済み）

3. **ActorCell の責務増加**:
   - ActorCell が Resume 契機の管理を担うため、タイマー管理やシグナル監視が必要
   - **対策**: Phase 2B で `ResumeCondition` に基づく薄いヘルパーを ActorCell に提供

### 影響を受けるコンポーネント

| コンポーネント | Phase 2B での変更内容 |
|----------------|----------------------|
| **MessageInvoker** | - `invoke_batch` 内で `actor_state()` を呼び出し<br>- `Suspended` の場合は `InvokeResult::Suspended` を返却 |
| **ReadyQueueCoordinator** | - （変更なし：Phase 0 で既に実装済み） |
| **ActorCell** | - `state` フィールドに `ActorState` を追加<br>- Resume 契機管理（タイマー、シグナル受信）<br>- `notify_resume()` メソッド追加 |
| **MailboxRegistry** | - `notify_resume(idx)` を受け取り `coordinator.register_ready(idx)` を呼び出す |

### 移行計画

#### Phase 0 (現在)
- ✅ `InvokeResult::Suspended` 定義（完了）
- ✅ `SuspendReason` / `ResumeCondition` 列挙型（完了）
- ✅ Coordinator の `handle_invoke_result` 実装（完了）

#### Phase 2B
1. `ActorCell` に `state: ActorState` フィールド追加
2. `MessageInvoker::invoke_batch` で状態チェック実装
3. `MailboxRegistry::notify_resume` 実装
4. ActorCell の Resume 契機管理（最小実装: `ResumeCondition::After` のみ）
5. 統合テスト: Suspend → Resume → 再処理

#### Phase 3
- `ResumeCondition::ExternalSignal` 実装（シグナルチャネル統合）
- `ResumeCondition::WhenCapacityAvailable` 実装（Mailbox 容量監視）

#### Phase 4
- Suspend/Resume のメトリクス追加（`suspend_count`, `resume_latency`）
- SuspendManager 導入の是非検討（複雑な Resume 条件が必要な場合）

### 検証方法

#### メトリクス
- Suspend/Resume サイクルのレイテンシ（目標: < 1ms）
- Resume 後の再処理開始までの時間（目標: < 10ms）

#### 成功基準
- Suspend 中のアクターが ready queue に存在しない
- Resume 後、未処理メッセージが正常に処理される
- Suspend/Resume 連続実行でメモリリークが発生しない

#### テスト方法

**単体テスト（Phase 2B）**:
```rust
#[test]
fn test_invoker_returns_suspended_when_actor_suspended() {
  let mut invoker = MockInvoker::new();
  invoker.set_actor_state(ActorState::Suspended);

  let result = invoker.invoke_batch(10);

  assert!(matches!(result, InvokeResult::Suspended { .. }));
}

#[test]
fn test_coordinator_unregisters_on_suspended() {
  let mut coordinator = DefaultReadyQueueCoordinator::new(32);
  let idx = MailboxIndex::new(1, 0);

  coordinator.register_ready(idx);
  coordinator.handle_invoke_result(idx, InvokeResult::Suspended {
    reason: SuspendReason::Backpressure,
    resume_on: ResumeCondition::WhenCapacityAvailable,
  });

  let mut out = Vec::new();
  coordinator.drain_ready_cycle(10, &mut out);
  assert_eq!(out.len(), 0); // idx は除外されている
}
```

**統合テスト（Phase 2B）**:
```rust
#[tokio::test]
async fn test_suspend_resume_cycle() {
  let system = ActorSystem::new();
  let actor = system.spawn(TestActor::new()).await;

  // Suspend 契機
  actor.send(SuspendMessage).await;
  tokio::time::sleep(Duration::from_millis(10)).await;

  // Suspend 中はメッセージ処理されない
  actor.send(TestMessage).await;
  assert!(!actor.has_processed(TestMessage));

  // Resume 契機
  actor.send(ResumeMessage).await;
  tokio::time::sleep(Duration::from_millis(10)).await;

  // Resume 後はメッセージ処理される
  assert!(actor.has_processed(TestMessage));
}
```

## 参照

### 関連ドキュメント

- [ADR-001: 命名ポリシー](./2025-10-22-phase0-naming-policy.md) - MessageInvoker の命名根拠
- [actor_scheduler_refactor.md](../design/actor_scheduler_refactor.md) - Section 7（オープン課題）
- [scheduler_message_flow.puml](../design/scheduler_message_flow.puml) - Suspend フローの図示

### 外部リンク

- [Akka Actor Lifecycle](https://doc.akka.io/docs/akka/current/typed/actor-lifecycle.html) - Akka の状態管理
- [Erlang gen_server:suspend/resume](https://www.erlang.org/doc/man/sys.html#suspend-1) - Erlang の Suspend 機構

## メタ情報

- **作成日**: 2025-10-22
- **最終更新日**: 2025-10-22
- **作成者**: Claude Code
- **レビュアー**: (未定)
- **関連 Issue/PR**: (未定)
- **対象フェーズ**: Phase 0（方針確定）、Phase 2B（実装）

---

## 更新履歴

| 日付 | 変更内容 | 変更者 |
|------|----------|--------|
| 2025-10-22 | 初版作成 | Claude Code |
