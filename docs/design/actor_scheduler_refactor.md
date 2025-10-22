# ActorScheduler ファサード再設計指針 (更新: 2025-10-21)

## 1. 背景
- 現行の `ActorScheduler` / `ReadyQueueScheduler` は multi-worker ready queue、Guardian 連携、エスカレーション配信、メトリクス・テレメトリ、receive timeout などの高度な機能を単一コンポーネントに抱え込んでいる。
- 旧 `dispatcher` 実装（`docs/sources/nexus-actor-rs/modules/actor-std/src/actor/dispatch/`）は Dispatcher / Mailbox / Invoker / Throttler へ責務を分割し SRP を徹底しており、protoactor-go を含む参照実装も同様の境界を持つ。
- Shared 抽象と async モデルへ移行する再設計の中で、ReadyQueueScheduler を facade とした責務再編を行い、再利用・テスト容易性を高めたい。
- 本ドキュメントは以下のドキュメント（旧: `actor_scheduler_mailbox_facade.md`, `D22-mailbox-readyqueue-facade.md`）を統合し、設計観点とロードマップを一貫させた最新版である。

## 2. 現状整理
1. `ReadyQueueScheduler` が Ready queue 管理、ワーカ駆動、Guardian 戦略、エスカレーション配信、Failure Telemetry、Metrics Sink、receive timeout 共有を併せ持ち、外向けファサードと内部オーケストレーションを兼任している。
2. `ActorCell` には mailbox 直接操作、メッセージ処理ループ、子アクター生成、サスペンド判定、Failure 通知といった Invoker 相当の責務が集中している。
3. `Mailbox` は `ActorCell` 経由でのみ所有されており、enqueue/notify と ReadyQueue 再登録、enqueue メトリクス記録に特化している。

| レイヤ | 主な型 | 現行責務 |
| --- | --- | --- |
| Mailbox | `QueueMailbox`, `QueueMailboxProducer` | enqueue/notify、ReadyQueue への再登録、enqueue メトリクス |
| Invoker 相当 | `ActorCell` | 優先度バッチ処理、System/User メッセージ分岐、サスペンド制御、Failure 伝播 |
| Dispatcher 相当 | `ReadyQueueScheduler`, `ReadyQueueWorkerImpl` | Ready index 管理、ActorCell の処理・再待機、ワーカ駆動 |
| ランタイム駆動 | `ActorSystemRunner`, `runtime_driver.rs` | ランタイムタスク生成、ワーカ数調整、shutdown 協調 |

## 3. 課題
- Mailbox ↔ Scheduler ↔ Invoker の境界が暗黙的で、API から意図が読み取りづらい。
- Suspend/Resume、middleware、詳細メトリクスなど旧機能が ActorCell／ReadyQueue に散在し拡張ポイントが不鮮明。
- ReadyQueueScheduler の内部構造が把握しづらく、Dispatcher/Invoker の概念が欠落しているため説明とテストが難しい。
- Shared 抽象や `no_std` 向け構成を想定したとき、現行の強結合がボトルネックになる。
- メトリクス、バックプレッシャ、receive timeout 等の TODO が複数ドキュメントに分散し、優先順位が不明瞭。

## 4. 目標アーキテクチャ

### 4.1 コンポーネント構成
1. **Mailbox Core**: QueueMailbox を中心に enqueue・シグナル通知・ReadyQueueHook 連携を担う純粋なデータ構造。バックプレッシャ閾値や middleware hook をオプション化する。
2. **Scheduler Facade**: ReadyQueueScheduler を外部 API の窓口としつつ、内部をサブコンポーネントへ分割。
   - `ReadyQueueDriver`: `drain_ready_cycle` / `wait_for_any_signal_future` 相当の ready queue 走査とワーカ調停を担当。
   - `MessageDispatcher`: ランタイム依存のタスク生成・ワーカ駆動・再スケジュール要求を扱う。
   - `MessageInvoker`: ActorCell に代わりメッセージ実行ループを抽象化し、Suspend/Resume や Guardian 通知を集中させる。
3. **Observability Hub**: Failure Telemetry / Metrics Sink / トレース送出を統一的に収集し、enqueue/dequeue の計測ポイントを整理する。
4. **Mailbox Registry**: Mailbox 生成・再利用・所有権管理を行い、spawn フローや再登録処理を簡素化する。
5. **Execution Drivers**: `ActorSystemRunner` や runtime driver が Dispatcher を経由してワーカ数調整・shutdown 協調を行う。Tokio / Embassy / テスト環境で共通トレイトを共有。

### 4.2 イベントフロー（案）
1. Producer が QueueMailbox へ enqueue し、シグナル通知で ReadyQueueHook を呼び出す。
2. ReadyQueueDriver が mailbox index を ready queue へ登録し、MessageDispatcher へ処理要求を渡す。
3. MessageDispatcher がランタイムタスクを生成し、MessageInvoker を実行する。
4. MessageInvoker が Envelope バッチ処理・Suspend/Resume 判定・Guardian/Telemetry 連携を担い、処理結果に応じて ReadyQueueDriver に再登録指示を返す。

### 4.3 責務境界ガイドライン
- Facade（ReadyQueueScheduler）は外部 API と内部コンポーネント初期化のみに注力し、実際の処理は Driver/Dispatcher/Invoker に委譲する。
- Mailbox Core はスレッド安全性と通知保証に専念し、業務ロジックを含まない。
- Observability Hub は enqueue/dequeue/エスカレーションなど全体の計測ポイントを一元管理し、個別コンポーネントからメトリクス実装を排除する。
- Mailbox Registry が lifecycle を束ねることで、ActorCell から mailbox 生成・破棄ロジックを切り離す。

## 5. フェーズ別ロードマップ
| フェーズ | 目標 | 主なタスク |
| --- | --- | --- |
| Phase 0 | 現状の境界を明文化し PoC の前提を固める | 内部コンポーネントの責務マッピング図作成、テレメトリ/metrics の現状整理、ReadyQueueScheduler 公開 API の維持方針を確認 |
| Phase 1 | Ready queue 処理の抽出 | `ReadyQueueDriver` の導入、`drain_ready_cycle` 等の移動、Facade から Driver への委譲・テスト整備 |
| Phase 2 | Dispatcher / Invoker の導入と旧機能再統合 | `MessageDispatcher`/`MessageInvoker` の雛形実装、Suspend/Resume・middleware・バックプレッシャ API 再配置、Guardian/エスカレーション通知の抽象化 |
| Phase 3 | Mailbox Registry と Observability Hub の整備 | Mailbox lifecycle の集中管理、enqueue/dequeue 計測の統一、Metrics Sink との連携強化、バックプレッシャ設定の外部化 |
| Phase 4 | ランタイム統合と命名再整理 | Dispatcher 経由のワーカ数チューニング API 公開、Tokio/Embassy driver 共通化、旧 Dispatcher モデルとの命名整合（`Dispatcher` への改名検討含む） |

## 6. 既存 TODO・関連ドキュメントとの整合
- `D14-mailbox-runtime-next-actions.md`: Send/Sync 境界精査、MailboxOptions 拡張、プリセット API、クロスビルド CI、メトリクス整備を Phase 2–3 のサブタスクとして取り込む。
- `D13-ready-queue-next-actions.md`: ワーカチューニング、Spawn ミドルウェア統合、観測ポイント強化を Dispatcher/Driver のロードマップに紐付け。
- `docs/design/archive/2025-10-13-mailbox-runtime-status.md`: QueueMailboxProducer の SingleThread 対応や metrics 仕上げを Phase 2 の継続課題として追跡。

## 7. オープン課題
- Suspend/Resume を ReadyQueue レベルで止めるか Invoker 内で完結させるかの判断。
- Middleware API を `PriorityMailboxSpawnerHandle` に再導入するか、Invoker / Registry 側で抽象化するか。
- MetricsSink を lock-free に保ちながら enqueue/dequeue の双方向測定を実現する戦略。
- Shared 抽象（`ArcShared` → `SharedDyn` 等）と `no_std` ターゲットでの互換性をどう設計するか。
- Facade 化によるレイヤ増加がレイテンシへ与える影響の計測プラン。

## 8. 次アクション（直近）
1. `ActorScheduler` 周辺のユースケース調査を補強し、責務マッピング図と API 境界図を作成する。
2. Phase 1 の `ReadyQueueDriver` PoC を別ブランチで実装し、ホスト向け単体テストを追加する。
3. `MessageInvoker` トレイトと ActorCell 抽出のドラフトを準備し、Suspend/Resume/Middleware の移管戦略をレビュー可能な形にする。

## 9. 成果物イメージ
- `ReadyQueueDriver` / `MessageDispatcher` / `MessageInvoker` のモジュール設計書とトレイト仕様書。
- QueueMailbox / ReadyQueueScheduler の API ドキュメント更新案。
- ランタイム driver（Tokio, Embassy, Local）に対する統合テストと運用ガイド。
