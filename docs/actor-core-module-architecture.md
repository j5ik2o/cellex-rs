# Actor Core モジュールアーキテクチャ

## 概要

`cellex-actor-core-rs` は3層のモジュールアーキテクチャで構成されています:

1. **`api/`** - パブリックAPI層(外部公開インターフェース)
2. **`internal/`** - 内部実装層(非公開実装詳細)
3. **`shared/`** - 共有ユーティリティ層(抽象化された共有型)

## 設計原則

### 1. レイヤー分離
- **api**: 外部ユーザー向けの安定したパブリックAPI
- **internal**: 実装詳細、パフォーマンス最適化、複雑なロジック
- **shared**: 複数レイヤーで使用される抽象化された型

### 2. 可視性ルール
- `api/*` モジュール → `pub` で公開、`lib.rs` で re-export
- `internal/*` モジュール → 原則非公開、一部の型のみ `lib.rs` で公開
- `shared/*` モジュール → 必要に応じて `lib.rs` で公開

### 3. 依存関係フロー
```
api → internal → shared
 ↓       ↓         ↓
外部利用 ← lib.rs (re-exports) ← crate root
```

## 現在のモジュール構成

### API層 (`api/`)

#### `api/actor` - アクター基本機能
**責務**: アクターの生成、参照、振る舞い、ライフサイクル管理

**サブモジュール**:
- `actor_ref` - ActorRef型(型付きアクター参照)
- `ask` - リクエスト/レスポンスパターン(Future-based)
- `behavior` - Behavior DSL(Akka Typed-style)
- `context` - アクターコンテキスト(メッセージ受信、子アクター管理)
- `failure` - アクター障害表現
- `props` - アクター生成プロパティ
- `root_context` - ルートコンテキスト(システムレベル操作)
- `shutdown_token` - シャットダウン同期
- `signal` - シグナルメッセージ

**設計課題**:
- `actor_ref` と `context` の責務が一部重複
- `failure` と `api/supervision/failure` の役割分担が不明確

#### `api/actor_system` - アクターシステム
**責務**: システム全体のライフサイクル、設定、起動/停止

**サブモジュール**:
- `actor_system` - ActorSystem本体
- `actor_system_builder` - ビルダーパターン
- `actor_system_config` - 設定管理
- `actor_system_runner` - ランタイム実行
- `spawn` - アクター生成トレイト
- `timer` - 時間操作抽象化

**設計課題**:
- `spawn` は `actor/props` と統合可能かもしれない

#### `api/actor_runtime` - ランタイム抽象化
**責務**: 環境別ランタイム(Tokio/Embedded)の抽象化

**サブモジュール**:
- `generic_actor_runtime` - ランタイムトレイト定義

**設計評価**: ✅ シンプルで明確

#### `api/mailbox` - メールボックス
**責務**: メッセージキュー、送受信、バックプレッシャー

**サブモジュール**:
- `mailbox_concurrency` - 並行性モード(ThreadSafe/SingleThread)
- `mailbox_handle` - メールボックスハンドル
- `mailbox_options` - 設定オプション
- `mailbox_producer` - 送信側インターフェース
- `mailbox_runtime` - ランタイム統合
- `mailbox_signal` - シグナル管理
- `messages/` - メッセージ型(SystemMessage, PriorityEnvelope, PriorityChannel)
- `queue_mailbox` - キューベースのメールボックス実装
- `queue_mailbox_producer` - キュー送信側実装
- `single_thread` - シングルスレッド並行性
- `thread_safe` - スレッドセーフ並行性

**設計課題**:
- `mailbox_producer` と `queue_mailbox_producer` の重複
- `messages/` サブディレクトリが必要か?(3ファイルのみ)

#### `api/messaging` - メッセージング
**責務**: メッセージエンベロープ、メタデータ、型消去メッセージ

**サブモジュール**:
- `dyn_message` - 型消去メッセージ(DynMessage)
- `dyn_message_value` - 型消去メッセージ値
- `message_envelope` - メッセージエンベロープ
- `message_metadata` - メッセージメタデータ
- `message_sender` - 送信者情報
- `metadata_storage` - メタデータストレージ
- `metadata_storage_mode` - ストレージモード
- `metadata_storage_record` - ストレージレコード
- `user_message` - ユーザーメッセージ型

**設計課題**:
- 9個のサブモジュールは多すぎる可能性
- `metadata_*` 系を統合できる
- `dyn_message` と `user_message` の関係が不明確

#### `api/supervision` - スーパービジョン
**責務**: 障害処理、エスカレーション、テレメトリ

**サブモジュール**:
- `escalation` - エスカレーション機構
- `failure` - 障害イベント表現
- `supervisor` - スーパーバイザー戦略
- `telemetry` - テレメトリ収集

**設計課題**:
- `api/actor/failure` との統合を検討すべき

#### `api/extensions` - 拡張機能
**責務**: 拡張ポイント、シリアライザーレジストリ

**サブモジュール**:
- `extension` - Extension trait
- `registry` - 拡張レジストリ
- `serializer_extension` - シリアライザー拡張

**設計評価**: ✅ シンプルで明確

#### `api/identity` - ID管理
**責務**: ActorId, ActorPath

**サブモジュール**:
- `actor_id` - アクターID
- `actor_path` - アクターパス

**設計評価**: ✅ シンプルで明確

#### `api/failure_event_stream` - 障害イベントストリーム
**責務**: 障害イベントの購読/通知

**設計課題**:
- `api/supervision` に統合すべき

### Internal層 (`internal/`)

#### `internal/actor` - アクター内部実装
**サブモジュール**:
- `actor_cell` - アクターセル(内部状態管理)
- `internal_props` - 内部Propsラッパー

**設計評価**: ✅ 責務明確

#### `internal/actor_system` - システム内部実装
**サブモジュール**:
- `internal_actor_system` - システム実装本体
- `internal_actor_system_config` - 内部設定
- `internal_root_context` - 内部ルートコンテキスト

**設計評価**: ✅ API層との対応明確

#### `internal/context` - コンテキスト内部実装
**サブモジュール**:
- `actor_context` - ActorContext実装
- `child_spawn_spec` - 子アクター生成仕様

**設計評価**: ✅ 責務明確

#### `internal/scheduler` - スケジューラー実装
**責務**: アクターのスケジューリング、タイムアウト管理

**サブモジュール**:
- `actor_scheduler` - Scheduler trait
- `child_naming` - 子アクター命名
- `immediate_scheduler` - テスト用即時スケジューラー
- `noop_receive_timeout_driver` - Noop timeout driver
- `noop_receive_timeout_scheduler` - Noop timeout scheduler
- `noop_receive_timeout_scheduler_factory` - Noop factory
- `ready_queue_scheduler/` - 準備キューベースのスケジューラー(7ファイル)
- `receive_timeout` - Receive timeout型
- `receive_timeout_scheduler` - Timeout scheduler
- `receive_timeout_scheduler_factory` - Timeout factory
- `receive_timeout/` - Timeout関連抽象化(2ファイル)
- `scheduler_builder` - Schedulerビルダー
- `scheduler_spawn_context` - Spawn context
- `spawn_error` - Spawn error

**設計課題**:
- **最も複雑なモジュール**: 14サブモジュール + 7ファイルのサブディレクトリ
- Noop系(3つ)は統合可能
- `receive_timeout` 関連の責務が分散(トップレベルとサブディレクトリ)
- `ready_queue_scheduler/` 内部が7ファイルで細分化されすぎ

#### `internal/mailbox` - メールボックス内部実装
**サブモジュール**:
- `priority_mailbox_builder` - 優先度mailboxビルダー
- `spawner` - Mailbox spawner
- `test_support/` - テストサポート(7ファイル)

**設計評価**: ✅ API層との分離明確

#### `internal/message` - メッセージ内部実装
**サブモジュール**:
- `internal_message_metadata` - 内部メタデータ
- `internal_message_sender` - 内部送信者
- `metadata_table` - メタデータテーブル
- `metadata_table_inner` - テーブル内部実装

**設計課題**:
- `api/messaging` との責務分担を明確化

#### `internal/metrics` - メトリクス
**サブモジュール**:
- `metrics_event` - メトリクスイベント
- `metrics_sink` - Sink trait
- `metrics_sink_shared` - 共有Sink
- `noop_metrics_sink` - Noop実装

**設計評価**: ✅ 責務明確

#### `internal/guardian` - ガーディアン戦略
**サブモジュール**:
- `always_restart` - 常に再起動戦略
- `child_record` - 子アクターレコード
- `guardian_strategy` - Guardian trait

**設計評価**: ✅ 責務明確

#### `internal/supervision` - スーパービジョン内部実装
**サブモジュール**:
- `composite_escalation_sink` - 複合エスカレーションシンク
- `custom_escalation_sink` - カスタムシンク
- `parent_guardian_sink` - 親ガーディアンシンク

**設計評価**: ✅ API層との分離明確

#### `internal/runtime_state` - ランタイム状態
**責務**: ランタイム状態管理

**設計評価**: ✅ シンプル

### Shared層 (`shared/`)

#### `shared/failure_telemetry` - 障害テレメトリ共有型
**サブモジュール**(6ファイル):
- `failure_event_handler_shared` - イベントハンドラー共有
- `failure_event_listener_shared` - イベントリスナー共有
- `failure_telemetry_builder_shared` - ビルダー共有
- `failure_telemetry_shared` - テレメトリ共有
- `telemetry_builder_fn` - ビルダー関数
- `telemetry_context` - コンテキスト

**設計課題**:
- 6ファイルは多すぎる可能性
- `*_shared` 命名が冗長

#### `shared/receive_timeout` - Receive timeout共有型
**サブモジュール**(4ファイル):
- `receive_timeout_driver` - Driver trait
- `receive_timeout_driver_bound` - Bound制約
- `receive_timeout_driver_shared` - Driver共有
- `receive_timeout_factory_shared` - Factory共有

**設計課題**:
- `internal/scheduler/receive_timeout` との重複感

#### `shared/map_system` - Map system共有型
**責務**: SystemMessage変換の共有型

**設計評価**: ✅ シンプル

## 主要な設計課題まとめ

### 1. スケジューラーの複雑性
**問題**: `internal/scheduler` が14サブモジュール + サブディレクトリで最も複雑

**提案**:
```
internal/scheduler/
├── core/           # actor_scheduler, scheduler_builder, spawn_context, spawn_error
├── ready_queue/    # ready_queue_scheduler/* (統合)
├── timeout/        # receive_timeout* 統合
└── noop/           # noop* 統合
```

### 2. メッセージング関連の重複
**問題**:
- `api/messaging` (9サブモジュール)
- `api/mailbox/messages` (3サブモジュール)
- `internal/message` (4サブモジュール)

**提案**: `api/messaging` に統合し、metadata関連をサブディレクトリ化

### 3. Failure処理の分散
**問題**:
- `api/actor/failure`
- `api/supervision/failure`
- `api/failure_event_stream`
- `shared/failure_telemetry`

**提案**: `api/supervision` 配下に統合

### 4. 共有型の命名冗長性
**問題**: `shared/*` 内で `*_shared` サフィックスが重複

**提案**: ディレクトリ名で明示的なので不要

### 5. 小さすぎるモジュール
**問題**:
- `api/identity` (2ファイルのみ)
- `api/actor_runtime` (1ファイルのみ)
- `shared/map_system` (1ファイルのみ)

**提案**: 関連モジュールに統合可能

## 理想的なモジュール構成

次のドキュメント `ideal-module-structure.md` で詳述
