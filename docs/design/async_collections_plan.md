# Async Collections Migration Plan

## 目的
- 同期 API を保持したまま、Tokio 等の async ランタイム上で自然に利用できる Queue / Stack API を追加する。
- 既存の Storage / Backend / TypeKey 抽象を再利用しつつ、`AsyncMutexLike` を通じて `await` ベースの API を公開する（`QueueStorage` / `StackStorage` に対して新たな `Async*Storage` トレイトは作成しない）。
- 旧 `Tokio` 系ラッパ（`ArcMpscBoundedQueue` 等）からの移行導線を提供する。

## 実装方針（段階的開発）

1. **Async API 土台の整備**
   - 既存の `ArcShared` と `AsyncMutexLike` をそのまま組み合わせる。新しい `AsyncShared` 系トレイトや `with_mut` などの派生 API は追加しない。
   - 擬似 async（同期バックエンドを `Future` で包む実装、コード名 `SyncAdapterBackend`）から着手し、真の async backend を後続フェーズで差し替えられるよう余地を残す。
   - `AsyncMutexLike` で割り込み文脈判定を共通化する `InterruptContextPolicy`（embedded では `critical_section::is_active` 等）を定義し、`SharedError::InterruptContext` が確実に上がるようにする。

2. **AsyncQueue / AsyncStack API**
   - 既存の `Queue<T, K, B, M>` / `Stack<T, B, M>` を async 版にラップする `AsyncQueue<T, K, B, A>` / `AsyncStack<T, B, A>` を追加。
   - API 例：
     ```rust
     async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError>
     async fn poll(&self) -> Result<T, QueueError>
     ```
   - `A = AsyncStdMutex<B>` は標準ランタイム（std + Tokio/async-std）向けのデフォルトとし、組込み向けには `CriticalSectionAsyncMutex` 等の代替を明示的に指定する。

3. **Tokio Backend 対応**
   - `TokioBoundedMpscBackend` 等を v2 backend として移植し、async API から利用可能にする。
   - `StdVecSyncStack` の async 版（`AsyncStdVecStack`）など、std 向けのデフォルト構成を整備する。
   - 擬似 async 実装をベースラインとして採用しつつ、Tokio/Embassy 等の真の async backend は後続フェーズで `async fn` を伴うトレイト拡張として導入する方針を明記する。

4. **互換 API の橋渡し**
   - 旧 `ArcMpscBoundedQueue` / `ArcStack` などから新 Async API への移行方法をドキュメント化。
   - `#[deprecated(note = "Prefer AsyncQueue/AsyncStack")]` などで開発者に切り替えを促す。

5. **検証とドキュメント**
   - `tokio::test` を使ったユニットテストで多Producer/SingleConsumer の挙動を確認。
   - `docs/guides` に async 版移行ガイドを追加。
   - CI では `cargo test --all-targets` に加え、`--features async` を有効にしたジョブを追加。thumb ターゲットでは async API はコンパイルのみ（Tokio が想定外のためテストはスキップ）。

## フェーズ構成

### フェーズ1: Async API の初期配置
ステータス: ✅ 完了（2025-10-24 時点）
- 追加の共有抽象は導入せず、`ArcShared` と `AsyncMutexLike` を直接組み合わせる。`ArcAsyncShared` や `AsyncSharedAccess` などの新ファイルは作成しない。
- ストレージ層は同期版と共通の `QueueStorage` / `StackStorage` を利用し、非同期専用のストレージトレイトを追加しない。
- **ディレクトリ構成は既存の `collections/queue`/`collections/stack` に async 系ファイルを責務別に並列配置する：**

  ```
  modules/utils-core/src/v2/collections/queue/
  ├─ backend/
  ├─ async_queue.rs            // 非同期 Queue API
  ├─ async_queue/              // 非同期 Queue の検証コード
  ├─ async_mpsc_producer.rs    // 非同期 MPSC プロデューサ
  ├─ async_mpsc_consumer.rs    // 非同期 MPSC コンシューマ
  ├─ async_spsc_producer.rs    // 非同期 SPSC プロデューサ
  ├─ async_spsc_consumer.rs    // 非同期 SPSC コンシューマ
  ├─ sync_queue.rs             // 同期 Queue API
  ├─ sync_mpsc_producer.rs     // 同期 MPSC プロデューサ
  ├─ sync_mpsc_consumer.rs     // 同期 MPSC コンシューマ
  ├─ sync_spsc_producer.rs     // 同期 SPSC プロデューサ
  ├─ sync_spsc_consumer.rs     // 同期 SPSC コンシューマ
  └─ tests.rs                  // 同期 Queue のユニットテスト
  └─ ...
  ```

- `utils-std` や `utils-embedded` も同じ階層に async 用ファイルを追加し、Tokio 等の環境依存実装はそこで提供する。

### フェーズ2: AsyncQueue / AsyncStack API
ステータス: ✅ 完了（2025-10-24 時点）
- `AsyncQueue<T, K, B, A>` を導入し、TypeKey と Capability を尊重した async API を提供。
- `SyncAdapterBackend` を用いた擬似 async 実装で offer/poll を `await` に対応させ、busy-wait ではなく `Notify` ベースの待機キューを実装する。
- `MpscKey` / `SpscKey` 用の async プロデューサ / コンシューマ (`AsyncMpscProducer` 等) を追加。
- Future の Drop（キャンセル）時に待機ノードを安全に除去できる RAII ガードを設計し、リークしないことを単体テストで確認する。

### フェーズ3: Tokio backend / std adapter
ステータス: ✅ 完了（2025-10-24 時点）
- `modules/utils-std/src/v2/collections/async_queue/` に Tokio backend 実装 (`TokioBoundedMpscBackend`) を追加し、`async_queue.rs` から公開する。
- `modules/utils-std/src/v2/` に `TokioMpscQueue` のビルダー関数を用意し、旧 API から async 版へ切り替えるための type alias / ガイドを整備。
- Embassy 連携は `modules/utils-embedded/src/v2/collections/async_queue.rs` で進め、`embassy-sync::channel::Channel` を backend 化する。
- **Backlog**: `std::sync::mpsc` を使った擬似 async backend を最初に揃え、`Tokio` 固有の真の async backend は後続フェーズで追加する。
- 利用者向けのデフォルト構成（例: `TokioMpscQueue<T>`）を型エイリアスとビルダーで提供し、複雑なジェネリクスを隠蔽する。

### フェーズ4: OverflowPolicy と ISR 方針の明文化
ステータス: ✅ 完了（2025-10-24 時点）
- `AsyncMutexLike::lock` を `Result<Guard, SharedError>` 返却に刷新し、割り込み文脈では `SharedError::InterruptContext` を伝搬させる。
- `utils-core::sync::interrupt::InterruptContextPolicy` を導入し、`AsyncMutexLike` 実装がロック前に `check_blocking_allowed()` を必ず呼ぶ構造を整える。標準環境は `NeverInterruptPolicy`、組込みは `CriticalSectionInterruptPolicy`（Cortex-M では `SCB::vect_active()` 判定）を採用できるようにする。
- `SpinAsyncMutexDefault` / `SpinAsyncMutexCritical` などの型エイリアスを提供し、利用者がポリシーを明示的に選択できる API を整備する。
- `OverflowPolicy::Block` は async 環境では `await` 待機に対応するが、割り込み文脈では同期版と同様に `QueueError::WouldBlock` / `StackError::WouldBlock` を即時返却することを仕様に反映する。
- `SharedError::BorrowConflict` → `WouldBlock` → `Poll::Pending` という写像を整理し、擬似 async / 真 async の両方で揃える。
- `InterruptContextPolicy` を thumb ターゲットで検証し、CI のクロスチェック（`cargo check --target thumbv6m-none-eabi --features async`）に組み込む。

### フェーズ5: 移行整備
ステータス: ✅ 完了（2025-10-24 時点）
- `docs/guides/async_queue_migration.md` を作成し、旧 Tokio ベース API からの移行手順を明示。
- 既存コードベース（actor-std 等）の移行実験を実施（`modules/actor-std/tests/async_queue_migration.rs`）。
- 追加の FAQ 整備など細部調整は必要に応じて随時対応。

### フェーズ6: 最終切り替え
ステータス: ⏳ 進行中（2025-10-24 時点）
- 互換 API に `#[deprecated]` を付与し、利用箇所の修正を促す。（完了）
- CI に async テストケースを追加し、ランタイム依存部分の regressions を防ぐ。（`cargo test -p cellex-actor-std-rs --tests` でカバー済み）
- 並行性検証用に `loom` を活用する PoC を Backlog に保持し、実装安定後に導入する（引き続き未着手）。
