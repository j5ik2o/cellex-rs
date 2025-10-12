# ActorScheduler 拡張プラン (2025-10-12)

## 目的
Shared ベースの `SchedulerBuilder`/`ActorScheduler` 抽象を導入したことで、PriorityScheduler 以外の実装を差し替え可能になった。次の段階として以下の要件を満たすスケジューラ群を設計・追加する。

1. **ImmediateScheduler (テスト向け) ✅**
   - 単一スレッド環境で即時にメッセージを実行する軽量実装。
   - `SchedulerBuilder::immediate()` を `cfg(test|test-support)` で提供済み（`PriorityScheduler` ラップ構成）。
   - 用途: 単体テスト／デバッグでの簡易実行。今後必要に応じて ReceiveTimeout 等を強化する。

2. **TokioScheduler (ホスト向け常駐タスク)**
   - `actor-std` クレートで `PriorityScheduler` をラップし、Tokio executor 上で協調切り替えする実装として提供。
   - 目的: Dispatcher を個別タスクに分離し、`ActorSystemRunner` を `await` せずとも runtime を駆動できるようにする。
   - 実装メモ:
     - `dispatch_next` の待機を `tokio::task::yield_now`／`Notify` 経由に。
     - Escalation や ReceiveTimeout は `PriorityScheduler` を内部にラップする Compose 型。

3. **EmbassyScheduler (no_std + async executor)**
   - `actor-embedded` クレートで `PriorityScheduler` をラップし、`embassy_futures::yield_now` による協調切り替えを提供する構成。
   - 目的: embedded/no_std でのマルチタスク実装を簡略化しつつ、`alloc` と `embassy` エコシステムのみで完結させる。
   - 追加要件: `Send` が使えない環境でも動作するよう trait 境界を調整し、`Rc`/`Arc` いずれの Shared 実装でも利用可能とする。
   - 詳細要件:
     1. `actor-embedded::embassy_scheduler_builder()` を提供し、`SchedulerBuilder::new` を通じて Embassy ラッパを生成。（✅ 実装済み）
     2. `ActorRuntimeBundleEmbassyExt::with_embassy_scheduler()` 経由でランタイム構築時の差し替えを簡素化（✅ 実装済み）。
     3. Mailbox 側は `LocalMailbox` を基本とし、`MailboxRuntime` の `Concurrency = SingleThread` パスを利用。
     4. ReceiveTimeout は `embassy_time::Timer` を想定。TimeoutDriver 抽象導入後はランタイムバンドル経由で差し替える。
     5. デバッグ／検証用に `embassy_executor::Executor::run(|spawner| ...)` を使った最小構成サンプルを追加し、クロスビルド (`thumbv6m-none-eabi`) が通ることを CI で保証。

## 実装スケジュール案
1. ImmediateScheduler
   - `modules/actor-core/src/runtime/scheduler/immediate_scheduler.rs` を追加。
   - テスト: `actor-core/src/runtime/scheduler/tests.rs` に `SchedulerBuilder::immediate()` 経由の動作確認を追加。
   - 追加 API: `SchedulerBuilder::immediate()` を `cfg(test)` および `feature = "std"` で公開。

2. TokioScheduler
   - `actor-std` に `TokioScheduler` と `tokio_scheduler_builder()` を配置。
   - 依存: `tokio::sync::Notify`、`tokio::task::yield_now`。
   - テスト: `tokio::test` を用いた統合テストを `modules/actor-std` 内に配置。

3. EmbassyScheduler
   - `modules/actor-embedded` に `scheduler` モジュールを追加し、`embassy_executor` feature で有効化。
   - `embassy_scheduler_builder()` を通じて `ActorSystem` 起動時に差し替え可能に。
   - テスト: `embassy_executor` の offline テストが難しいため、サンプル／ベンチで検証。

## 既知の課題
- `ActorScheduler` トレイトのオプション API（Escalation など）は default 実装を提供済みだが、各スケジューラで適宜 override する必要がある。
- `InternalActorRef` が `MailboxRuntime::Producer` 境界に `RuntimeBound` を要求しているため、no_std で `Rc` を使う場合の Send/Sync 条件を調整する必要がある。EmbassyScheduler 実装時に合わせて検証する。
- `Shared` のフォールバック戦略（ArcShared/RcShared）の動作確認が不足している。`ActorRuntimeBundle` 側で Shared を統一使用する流れに沿って、Runtime 側の API からも Shared をそのまま受け渡す設計を徹底する。
- `ActorSystemBuilder` に Scheduler 差し替え API を公開するかどうかは Step 3 で再検討。

## 次のアクション
1. ImmediateScheduler のコード実装→テスト整備。
2. TokioScheduler の設計検証・PoC。
3. Embedded 用スケジューラの要件整理（Send/Sync 境界、`Shared` バックエンド切替）。
4. RuntimeBundle に `scheduler_builder` を差し替えるパブリック API を提供するか要件確認。
