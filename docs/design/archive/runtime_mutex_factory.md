# Runtime Mutex 抽象化設計

## 背景

- 現状の `Props` や ready queue 実装では `spin::Mutex` を直接生成しており、Tokio ランタイムなど `std`/async 環境との相性が悪い。
- 「ランタイムの種類に応じた最適な mutex 実装（spin/std/tokio）を使う」方針にしたいが、利用者に `ActorRuntime` を渡させる API は UX を損なう。
- 利用側（`Props::new` 等）は従来どおり簡潔に書けるようにしつつ、内部ではランタイムに応じた mutex を差し込める仕組みが必要。

## 要求仕様

1. `Props::new(|_, msg| { ... })` など従来の呼び方を維持し、ユーザーは runtime を意識しない。
2. ランタイムが spin/std/tokio のいずれかの mutex を生成できるようにする。
3. `ActorContext` や `InternalProps` などアクター内部からもランタイム依存の mutex を生成できるようにする。
4. `no_std` / `std` / Tokio などの環境差異を隠蔽し、将来の差し替え（例えば `parking_lot`）にも柔軟に対応できるようにする。
5. ランタイム構築時に使用する mutex 実装を選択できるようにする。

## 基本構成

### 1. 関連型による Mutex 抽象化

`ActorRuntime` に以下のような関連型を追加:

```rust
trait ActorRuntime {
  type SyncMutex<T>: SyncMutexLike<T>;
  type AsyncMutex<T: Send>: AsyncMutexLike<T>;
  // ...
}
```

ポイント:
- ランタイム毎に適切な mutex 型を関連型として定義
- コンパイル時に型が決定されるため、実行時オーバーヘッドなし
- `no_std` 環境では `SpinSyncMutex` / `SpinAsyncMutex`
- `std` 環境では `StdSyncMutex` / `TokioAsyncMutex`

### 2. ランタイム構築時の Mutex 選択

将来的に、ランタイム構造体をジェネリックにすることで mutex 実装を選択可能に:

```rust
// デフォルト: StdSyncMutex を使用
struct TokioActorRuntime<M = StdSyncMutex> {
  _mutex: PhantomData<M>,
  // ...
}

// カスタム: SpinSyncMutex を使用
let runtime = TokioActorRuntime::<SpinSyncMutex>::new(...);
```

### 3. `Props` などでの使用

`Props` は `ActorRuntime` の関連型を直接使用:

```rust
impl<U, AR: ActorRuntime> Props<U, AR> {
  pub fn new<F>(handler: F) -> Self {
    let handler_cell = ArcShared::new(AR::SyncMutex::new(handler));
    // ...
  }
}
```

### 4. 既存の `spin::Mutex` 使用箇所の整理

- `Props` 初期化: `AR::SyncMutex::new()` を使用
- ランタイム固有実装 (TokioMailbox等): 具体的なラッパー型を直接使用
- テストコード: 環境に応じた適切なラッパーを使用

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

## 実装状況

### 完了項目

1. **共通トレイトとラッパ型の整備** ✅
   - `SyncMutexLike` / `AsyncMutexLike` トレイトは既存
   - `SpinSyncMutex` / `SpinAsyncMutex` (utils-core) は既存
   - `StdSyncMutex` / `TokioAsyncMutex` (utils-std) は既存

2. **ActorRuntime拡張** ✅
   - `ActorRuntime` トレイトに関連型 `SyncMutex<T>` / `AsyncMutexLike<T: Send>` を追加
   - `GenericActorRuntime` で各ランタイムに応じた実装を提供:
     - 関連型が `feature = "std"` により自動的に切り替わる
     - `no_std`: `SpinSyncMutex` / `SpinAsyncMutex`
     - `std`: `StdSyncMutex` / `TokioAsyncMutex`
   - コンパイル時に型が決定されるため、実行時オーバーヘッドなし

4. **ビルド・テスト確認** ✅
   - `ci-check.sh` で全構成 (no_std, std, tokio) のビルド・テストが成功
   - `thumbv6m-none-eabi` などの組み込みターゲットでもビルド成功
   - ドキュメント生成も問題なし

### 実装完了項目

5. **既存コードの置き換え** ✅
   - `Props` (modules/actor-core/src/api/actor/props.rs)
     - `spin::Mutex` から `AR::SyncMutex` (関連型経由) に変更
     - `Props::new` および `Props::with_system_handler` で適用
   - `TokioPriorityMailbox` queues (modules/actor-std/src/tokio_priority_mailbox/queues.rs)
     - `std::sync::Mutex` から `StdSyncMutex` ラッパーに変更
     - `TokioPriorityLevels` および `TokioPriorityQueues` で適用
   - すべてのテストがパス、ci-check成功

### 設計判断

**関連型のみの採用**

- **静的構築時 (Props等)**: `AR::SyncMutex::new()` を直接使用
  - コンパイル時に型が決定されるため、実行時オーバーヘッドなし
  - 関連型により適切なmutex実装が静的に解決される

- **ランタイム固有実装 (TokioMailbox等)**: 具体的なラッパー型を直接使用
  - 特定のランタイム専用の実装であり、動的な切り替えが不要
  - 例: `StdSyncMutex`, `TokioAsyncMutex`

- **Factory関数は不要**:
  - 当初はfactory関数による動的生成を検討したが、関連型だけで要件を満たせることが判明
  - ランタイム構築時にmutex実装を選択したい場合は、ランタイム構造体をジェネリック化すればよい
  - 例: `TokioActorRuntime<M = StdSyncMutex>` のように型パラメータで指定

### 将来の拡張可能性

**ランタイム構造体のジェネリック化** (将来の拡張として)

```rust
struct GenericActorRuntime<MF, SM = SpinSyncMutex, AM = SpinAsyncMutex> {
  _sync_mutex: PhantomData<SM>,
  _async_mutex: PhantomData<AM>,
  // ...
}

impl<MF, SM, AM> ActorRuntime for GenericActorRuntime<MF, SM, AM>
where
  SM: SyncMutexLike<T>,
  AM: AsyncMutexLike<T>,
{
  type SyncMutex<T> = SM;
  type AsyncMutex<T: Send> = AM;
  // ...
}
```

これにより、同じランタイムでも異なるmutex実装を選択可能に。
