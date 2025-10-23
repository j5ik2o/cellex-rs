# Async Collections Migration Plan

## 目的
- 同期 API を保持したまま、Tokio 等の async ランタイム上で自然に利用できる Queue / Stack ファサードを追加する。
- 既存の Storage / Backend / TypeKey 抽象を再利用しつつ、`AsyncMutexLike` を通じて `await` ベースの API を公開する。
- 旧 `Tokio` 系ラッパ（`ArcMpscBoundedQueue` 等）からの移行導線を提供する。

## 実装方針（段階的開発）

1. **Async ファサード土台の整備**
   - 既存の `ArcShared` と `AsyncMutexLike` をそのまま組み合わせる。新しい `AsyncShared` 系トレイトや `with_mut` などの派生 API は追加しない。
   - 擬似 async（同期バックエンドを `Future` で包む実装、コード名 `SyncAdapterBackend`）から着手し、真の async backend を後続フェーズで差し替えられるよう余地を残す。

2. **AsyncQueue / AsyncStack ファサード**
   - 既存の `Queue<T, K, B, M>` / `Stack<T, B, M>` を async 版にラップする `AsyncQueue<T, K, B, A>` / `AsyncStack<T, B, A>` を追加。
   - API 例：
     ```rust
     async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError>
     async fn poll(&self) -> Result<T, QueueError>
     ```
   - `A = AsyncStdMutex<B>` は標準ランタイム（std + Tokio/async-std）向けのデフォルトとし、組込み向けには `CriticalSectionAsyncMutex` 等の代替を明示的に指定する。

3. **Tokio Backend 対応**
   - `TokioBoundedMpscBackend` 等を v2 backend として移植し、async ファサードから利用可能にする。
   - `StdVecStack` の async 版（`AsyncStdVecStack`）など、std 向けのデフォルト構成を整備する。
   - 擬似 async 実装をベースラインとして採用しつつ、Tokio/Embassy 等の真の async backend は後続フェーズで `async fn` を伴うトレイト拡張として導入する方針を明記する。

4. **互換 API の橋渡し**
   - 旧 `ArcMpscBoundedQueue` / `ArcStack` などから新 Async API への移行方法をドキュメント化。
   - `#[deprecated(note = "Prefer AsyncQueue/AsyncStack")]` などで開発者に切り替えを促す。

5. **検証とドキュメント**
   - `tokio::test` を使ったユニットテストで多Producer/SingleConsumer の挙動を確認。
   - `docs/guides` に async 版移行ガイドを追加。
   - CI では `cargo test --all-targets` に加え、`--features async` を有効にしたジョブを追加。thumb ターゲットでは async API はコンパイルのみ（Tokio が想定外のためテストはスキップ）。

## フェーズ構成

### フェーズ1: Async ファサードの初期配置
- 追加の共有抽象は導入せず、`ArcShared` と `AsyncMutexLike` を直接組み合わせる。`ArcAsyncShared` や `AsyncSharedAccess` などの新ファイルは作成しない。
- **ディレクトリ構成は既存の `collections/queue`/`collections/stack` に async 系ファイルを並列配置する：**

  ```
  modules/utils-core/src/v2/collections/queue/
  ├─ backend/
  ├─ facade/
  │   ├─ queue.rs              // 同期ファサード
  │   ├─ mpsc_producer.rs      // 同期 MPSC ハンドル
  │   ├─ async_queue.rs        // 非同期ファサード（新規）
  │   └─ async_mpsc_producer.rs 等
  └─ ...
  ```

- `utils-std` や `utils-embedded` も同じ階層に async 用ファイルを追加し、Tokio 等の環境依存実装はそこで提供する。

### フェーズ2: AsyncQueue / AsyncStack ファサード
- `AsyncQueue<T, K, B, A>` を導入し、TypeKey と Capability を尊重した async API を提供。
- `SyncAdapterBackend` を用いた擬似 async 実装で offer/poll を `await` に対応させ、真の async backend への差し替えが可能な構造を確立する。
- `MpscKey` / `SpscKey` 用の async ハンドル (`AsyncMpscProducer` 等) を追加。

### フェーズ3: Tokio backend / std adapter
- `modules/utils-std/src/v2/` に `AsyncStdMpscQueue` / `AsyncStdVecStack` などのビルダー関数を用意。
- 旧 API から async 版へ切り替えるための type alias / ガイドを整備。
- **Backlog**: `std::sync::mpsc` を使った擬似 async backend を最初に揃え、`Tokio` 固有の真の async backend は後続フェーズで追加する。

### フェーズ4: OverflowPolicy と ISR 方針の明文化
- `OverflowPolicy::Block` は async 環境では `await` 待機に対応するが、割り込み文脈では同期版と同様に `QueueError::WouldBlock` / `StackError::WouldBlock` を即時返却することを仕様に反映する。
- `SharedError::BorrowConflict` → `WouldBlock` → `Poll::Pending` という写像を整理し、擬似 async / 真 async の両方で揃える。

### フェーズ5: 移行整備
- `docs/guides/async_queue_migration.md` を作成し、旧 Tokio ベース API からの移行手順を明示。
- 既存コードベース（actor-std 等）の移行実験。

### フェーズ6: 最終切り替え
- 互換 API に `#[deprecated]` を付与し、利用箇所の修正を促す。
- CI に async テストケースを追加し、ランタイム依存部分の regressions を防ぐ。
