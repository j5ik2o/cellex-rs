# Runtime Mutex Factory 設計

## 背景

- 現状の `Props` や ready queue 実装では `spin::Mutex` を直接生成しており、Tokio ランタイムなど `std`/async 環境との相性が悪い。
- 「ランタイムの種類に応じた最適な mutex 実装（spin/std/tokio）を使う」方針にしたいが、利用者に `ActorRuntime` を渡させる API は UX を損なう。
- 利用側（`Props::new` 等）は従来どおり簡潔に書けるようにしつつ、内部ではランタイムに応じた mutex を差し込める仕組みが必要。

## 要求仕様

1. `Props::new(|_, msg| { ... })` など従来の呼び方を維持し、ユーザーは runtime を意識しない。
2. ランタイムが spin/std/tokio のいずれかの mutex を生成できるようにする。
3. `ActorContext` や `InternalProps` などアクター内部からもランタイム依存の mutex を生成できるようにする。
4. `no_std` / `std` / Tokio などの環境差異を隠蔽し、将来の差し替え（例えば `parking_lot`）にも柔軟に対応できるようにする。

## 基本構成

### 1. Factory クロージャを `ActorRuntime` から提供

`ActorRuntime` に以下のような関連型と factory アクセサを追加するイメージ。

```rust
type SyncMutex<T>: SyncMutexLike<T>;
type AsyncMutex<T>: AsyncMutexLike<T>;

fn sync_mutex_factory(&self) -> Arc<dyn Fn<T>(T) -> Self::SyncMutex<T> + Send + Sync>;
fn async_mutex_factory(&self) -> Arc<dyn Fn<T>(T) -> Self::AsyncMutex<T> + Send + Sync>;
```

ポイント:
- 関数ポインタではなく `Arc<dyn Fn>` を返すことで、クロージャ内で `Arc` クローンが自由にでき、スレッド間共有も安全。
- ランタイム毎に適切な型（`SpinMutex` など）を包んだクロージャを提供する。
- `AsyncMutex` は Tokio など async ランタイム向けで、`no_std` 環境では `SpinAsyncMutex` を返す。

### 2. Factory を `ActorSystem` / `ActorContext` に注入

- `GenericActorSystem::new_with_actor_runtime` など、アクターシステム生成時にランタイムから factory を取得し、`InternalActorContext` などへ保持させる。
- `ActorContext` 内部に `Arc<dyn Fn<T>(T) -> MutexType>` の参照を持たせ、ハンドラ内で `ctx.make_sync_mutex(value)` のように呼び出せる API を提供。
- これにより `Props` やハンドラは「ランタイムが提供する factory」を介して mutex を生成できる。

### 3. `Props` の API を維持

- `Props::new` / `Props::with_behavior` は従来通りハンドラクロージャを受け取り、内部で mutex を直接生成しない。
- 内部状態をロック付きで持ちたい場合は、ハンドラ内またはアダプタ初期化時に `ctx.make_sync_mutex` を使って生成する。

### 4. 既存の `spin::Mutex` 使用箇所の整理

- ready queue、`Props` 初期化、各テスト等で直接 `spin::Mutex::new` が呼ばれている箇所を、 factory 経由のロック生成に置き換える。
- `no_std` と `std` の動作差異を吸収するため、`SpinSyncMutex` / `StdSyncMutex` / `TokioAsyncMutex` などの薄いラッパ型を `utils-core` / `utils-std` に整備する。

## 実装ステップ

1. **共通トレイトとラッパ型の整備**     - `utils-core` に `SyncMutexLike` / `AsyncMutexLike` を定義し、`SpinSyncMutex` / `SpinAsyncMutex` を実装。     - `utils-std` に `StdSyncMutex`, `TokioAsyncMutex` を追加し、各トレイトを実装。     - 単体テスト: `cargo check -p cellex-utils-core-rs` / `cellex-utils-std-rs`。

2. **ActorRuntime 拡張**     - 既存トレイトに関連型 `SyncMutex<T>` / `AsyncMutex<T>` と factory アクセサ `sync_mutex_factory()` / `async_mutex_factory()` を追加。     - `GenericActorRuntime` / `TokioActorRuntime` / 組み込みランタイムで適切なラッパを返す実装を追加。     - テスト: `cargo check -p cellex-actor-core-rs`（`alloc`/`std` 両構成）。

3. **Factory の注入**     - `GenericActorSystem::new_with_actor_runtime` などで factory クロージャを取得し、`InternalActorContext` に保持させる。     - `ActorContext` に `fn make_sync_mutex<T>(&self, value: T)` / `make_async_mutex` のヘルパを追加。     - 影響範囲: `ActorContext` 初期化、拡張の再注入、ライフタイム調整。

4. **既存コードの置き換え**     - `Props` / ready queue / mailboxes / テストなどで `spin::Mutex::new` を直接呼んでいる箇所を、上記ヘルパ経由に差し替え。     - パターン: ハンドラ定義 → `ctx.make_sync_mutex`, 状態共有 → factory から生成。     - ブランチごとに `cargo fmt` + `cargo test` を実行。

5. **コンパチ確認**     - `makers ci-check` で全構成 (no_std, std, tokio) のビルド／テストを実行。     - ランタイムごとの挙動 (Tokio: deadlock 解消、組み込み: ビルド可) を確認。

6. **ドキュメント更新**     - 本設計ファイルの最終版、`synchronization-abstraction.md` などを更新。     - 変更点・利用方法を README もしくはガイドに追記。

## 想定される課題

- factory クロージャを大量に複製すると `Arc` クローンが増えコストになる → 基本的に `Arc` 1本をコンテキスト内で共有すれば可。
- `AsyncMutex` を使用する箇所と同期的な箇所の整理 → 既存コードのロック使用箇所を分類し、それぞれ適切な factory を選択する。
- ライフタイム管理 → factory は `Arc` に包むことで `'static` な関数オブジェクトとして扱い、lifetimes を簡潔に保つ。

## まとめ

- ユーザー API (`Props::new`, `ActorRef::tell` など) を変更せず、内部で最適な mutex を使い分けるには「ランタイムが factory を提供し、コンテキストやアダプタがそれを使う」設計が有効。
- 追加で必要なのは、mutex ラッパと factory 注入の仕組み、そして既存 `spin::Mutex` 使用箇所の置き換え。
- この設計により、Tokio ランタイム上でのデッドロックや CPU ビジーを避けつつ、組み込み環境の `no_std` 互換性も維持できる。
