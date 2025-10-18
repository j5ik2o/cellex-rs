# ActorScheduler リファクタリング方針メモ (2025-10-18)

## 背景
- 現行の `ActorScheduler` / `ReadyQueueScheduler` は multi-worker ready queue、Guardian 連携、エスカレーション配信、メトリクス・テレメトリ、receive timeout など高度な機能を単一コンポーネントに集約している。
- 旧 `dispatcher` 実装（`docs/sources/nexus-actor-rs/modules/actor-std/src/actor/dispatch/`）は責務ごとに Dispatcher / Mailbox / Invoker / Throttler へ分割されており、Single Responsibility Principle を徹底していた。
- 現行機能を維持しつつ SRP を再適用できれば、保守性を上げながら将来的なリネーム (`Dispatcher` への名称変更など) にも備えられる。

## 現状サマリ
1. `ReadyQueueScheduler` が以下を全て担っている:
   - Ready queue の管理とワーカ調整 (`drain_ready_cycle`, `wait_for_any_signal_future`)
   - Guardian 戦略 / 監視登録 / エスカレーション再配送
   - Failure Telemetry / Metrics Sink の中継
   - Receive timeout ファクトリ共有
2. `ActorCell` も mailbox 直接操作・コンテキスト生成・子アクター登録・Failure 通知まで多責務化している。
3. `Mailbox` は `ActorCell` 以外からは所有されていないため、インスタンスを扱う箇所は `ActorCell` と spawn 経路に限定できる。

## リファクタリング指針（アイデア段階）
1. **ReadyQueueScheduler を中核にした責務分解**
   - 既存の `ReadyQueueScheduler` を “外向けファサード” と位置づけ、内部構造を段階的にコンポーネントへ分解する。
   - 外部 API（`ActorSchedulerHandleBuilder` 等）は現状維持とし、内部構造のみ整理する。
2. **責務ごとのコンポーネント化**
   - Ready queue 駆動 (`ReadyQueueDriver`)
   - Guardian / エスカレーション (`EscalationRouter`)
   - Telemetry / Metrics (`TelemetryHub`, `MetricsCollector`)
   - Mailbox 登録 / 再利用 (`MailboxRegistry`)
   - これらを `ReadyQueueScheduler` 内部で組み合わせる構造へ移行し、実装詳細を小さなモジュールへ隔離する。
3. **ActorCell の軽量化**
   - メッセージ取得・ハンドラ呼び出し・子 spawn を小さなヘルパーへ委譲し、テスト可能な単位に分割。
   - Guardian への通知やエスカレーション積み上げは `EscalationRouter` に委譲できるようにインタフェース化。
4. **旧 Dispatcher モデルとのすり合わせ**
   - 旧 `dispatcher` ディレクトリを参考に、Runnable 実行層 (`Dispatcher`)、Invoker 層、Mailbox 層の境界を再定義。
   - `no_std` / `Shared<T>` 抽象と両立するよう、Tokio / Embassy / テストサポートに共通のトレイトを用意する。
5. **ドキュメントと段階的移行**
   - 各フェーズで `docs/design` に Update を追記し、移行中の依存関係とテスト手順を明確化。
   - 段階的に `mailbox` の公開 API を縮小し、最終的にファサード or 専用マネージャからのみ mailbox インスタンスを扱う。

## フェーズ案（ドラフト）
1. **Phase 0**: `ReadyQueueScheduler` の公開 API は維持したまま、内部に配置する構造体/モジュールの分割方針をドキュメント化。
2. **Phase 1**: Ready queue 処理 (`drain_ready_cycle`, `wait_for_any_signal_future`) を `ReadyQueueDriver`（仮）に抽出し、`ReadyQueueScheduler` はドライバへ処理を委譲する。
3. **Phase 2**: Escalation / Guardian / Telemetry を専用コンポーネントへ切り出し、`ActorCell` の `dispatch_envelope` から新コンポーネント経由で通知。
4. **Phase 3**: Mailbox 生成と登録を `MailboxRegistry`（仮称）へ移し、`ActorCell` はハンドラ実行に専念。子 spawn フローも再整理。
5. **Phase 4**: 旧 Dispatcher モデルに合わせた命名/モジュール構成へリネーム検討（必要なら `Dispatcher` への改名を再評価）。

## 残課題
- ファサード化で追加されるレイヤーのオーバーヘッド計測。
- `no_std` 環境での Shared 抽象をどう扱うか（特に `ArcShared` → `SharedDyn` の扱い）。
- テレメトリやメトリクスのライフサイクル管理を新コンポーネントに移した際の API 互換性。
- フェーズごとのテスト戦略（既存ユニットテストの再利用と欠落テストの洗い出し）。

## 次ステップ（短期 TODO）
1. 使用パターン調査結果を詳細化し、`ActorScheduler` 周辺の責務マッピング図を作成する。
2. Phase 0 のファサード実装案を検討し、PoC コードを別ブランチで作成。
3. `ActorCell` のテストを追加し、リファクタ後も挙動を検証できるよう準備する。
