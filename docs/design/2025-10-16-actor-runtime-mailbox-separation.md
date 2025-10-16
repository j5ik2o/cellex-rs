# ActorRuntime / MailboxRuntime 分離計画 (2025-10-16)

## 背景
- これまで `ActorRuntime` は `MailboxRuntime` を継承していたため、ランタイム利用側が「メールボックス関連 API」も同一トレイトで扱う前提になっていた。
- `GenericActorRuntime` などが `MailboxRuntime` を内包している設計では、`ActorRuntime` を “ランタイムプリセットのファサード” として扱いたいが、現状は `ActorRuntime + MailboxRuntime` の二重制約が至るところで必要になっている。
- `MailboxHandleFactoryStub` の撤廃により、スケジューラには実際の `MailboxRuntime` を直接渡す道筋ができたため、両トレイトを明確に分離し、`ActorRuntime` からは `mailbox_runtime()` / `mailbox_runtime_shared()` を通じてアクセスさせる構成に揃えたい。

## 目標
- ジェネリック境界で `ActorRuntime + MailboxRuntime` を同時に要求しないようにする。
- `ActorRuntime` を受け取る側は `MailboxOf<R> = <R as ActorRuntime>::Mailbox` のエイリアスを介して、必要な関連型（Queue / Signal / Producer / Concurrency など）にアクセスする。
- 実行時に `MailboxRuntime` の機能が必要な箇所は、`ActorRuntime::mailbox_runtime()` もしくは `ActorRuntime::mailbox_runtime_shared()` を必ず経由する。

## ステップ
1. **共通エイリアスの整備**
   - `runtime/mailbox/traits.rs` に `pub type MailboxOf<R> = <R as ActorRuntime>::Mailbox;` を追加済み。
   - 各モジュールで `MailboxOf<R>` を用いた関連型参照に切り替える。

2. **上位 API (`ActorSystem` / `ActorSystemBuilder` / `ActorSystemRunner`) の修正**
   - `R: ActorRuntime + Clone` のみを要求し、必要な関連型は `MailboxOf<R>` を通じて参照する。
   - `InternalActorSystem` ではランタイムとメールボックスランタイムの両ハンドルを保持 (`ArcShared<R>` と `ArcShared<MailboxOf<R>>`)。

3. **スケジューラ層の更新**
   - `SchedulerSpawnContext` に `mailbox_runtime: ArcShared<R>` を追加済みなので、`ReadyQueueScheduler` / `ImmediateScheduler` などで `PriorityMailboxSpawnerHandle::new(mailbox_runtime)` を用いる実装へ統一。
   - `ActorCell` / `InternalActorRef` など、メールボックス操作が必要な箇所は `MailboxOf<R>` を介した関連型へ書き換える。

4. **API コンテキスト (`Context`, `Props`, `ActorRef`, `Behavior`, `RootContext`) の更新**
   - それぞれの `where` 節を `MailboxOf<R>` 形式へ変更し、内部で `mailbox_runtime()` を呼んでいた箇所は適宜差し替える。
   - `InternalProps` など、内部データ構造も `MailboxOf<R>` を使うように調整。

5. **周辺クレートへの反映**
   - `actor-std` / `actor-embedded` / テストコードでも同様に `MailboxOf` を用いた境界へ切り替え。

6. **サマリ・ドキュメントの更新**
   - `2025-10-15-actor-runtime-refactor-plan.md` など既存の設計メモに「Stub 撤廃後は `MailboxOf<R>` を用いる」旨を追記する。

## 留意点
- 型制約は冗長になりがちなので、必要に応じてローカルの `type` エイリアス（例: `type MailboxRT<R> = MailboxOf<R>;`）で読みやすさを確保する。
- `GenericActorRuntime<R>` は引き続き `MailboxRuntime` を実装しているが、利用側は `ActorRuntime` から `MailboxOf` を辿る形でアクセスする。
- 大規模な境界変更となるため、各ステップで `cargo check` / `cargo test --workspace` を回しながら段階的に移行する。

## 設計詳細

### トレイト境界と関連型
- `ActorRuntime` は `MailboxRuntime` を関連型として保持し、`mailbox_runtime()` / `mailbox_runtime_shared()` のトレイトメソッド経由でのみアクセスさせる。
- 関連型の参照は `MailboxOf<R>` を通すことで、利用側が `MailboxRuntime` に直接依存しなくても必要な型情報に到達できる。
- 想定シグネチャ:

```rust
pub trait ActorRuntime: Send + Sync + 'static {
  type Mailbox: MailboxRuntime<Self>;
  type Scheduler: SchedulerRuntime<Self>;

  fn mailbox_runtime(&self) -> &Self::Mailbox;
  fn mailbox_runtime_shared(&self) -> ArcShared<Self::Mailbox>;
}
```

### MailboxRuntime 実装契約
- `MailboxRuntime<R>` は `enqueue`, `schedule_dispatch`, `poll_mailbox` 等の主要操作を `R` のスケジューラ契約と連携する責務を持つ。
- `ArcShared<Self>` を返す `shared()` メソッドを共通化し、`mailbox_runtime_shared()` と型整合を取る。
- `protoactor-go` の `actor/mailbox.go` における `Dispatcher` / `Mailbox` 分離を参考にしつつ、Rust では `Send + Sync` の境界と `Pin<&Self>` の扱いを明確にする。

### 共有ハンドル設計
- `ActorRuntime` 自体を `ArcShared<R>` として扱うのに加えて、`MailboxOf<R>` も同じ `ArcShared` で共有する。
- `InternalActorSystem` は以下のようなフィールド構成を想定:

```rust
pub struct InternalActorSystem<R>
where
  R: ActorRuntime,
{
  runtime: ArcShared<R>,
  mailbox_runtime: ArcShared<MailboxOf<R>>,
  scheduler: ArcShared<R::Scheduler>,
}
```

- スケジューラ側は `ArcShared<MailboxOf<R>>` を直接受け取ってメールボックス生成・スケジュールを行う。

### API 遷移例
- `ActorCell<'a, R>` では `where R: ActorRuntime` のみを残し、`let mailbox_rt = self.system.mailbox_runtime();` ではなく `let mailbox_rt = self.system.mailbox_runtime_shared();` を取得して共有参照を保持する。
- `ActorRef<R>` も `MailboxProducerOf<R>` などの型エイリアスを通して `MailboxOf<R>` にアクセスし、直接的な `MailboxRuntime` 境界を外す。
- `Context` 系 API はメールボックス操作が必要な箇所だけ `MailboxOf<R>` を通じて `EnqueueHandle` 等を取り出す形に収束させる。

## 移行計画の詳細

### フェーズ A: 型境界の導入
- 各モジュールで `MailboxOf<R>` を導入し、`where` 句から `MailboxRuntime` 直接参照を撤廃。
- 互換性が崩れる箇所は一旦 `todo!()` を置かず、既存の `MailboxRuntime` 実装をそのまま呼び出せるように順次書き換える。

### フェーズ B: ハンドル配線の整理
- `ActorSystemBuilder` に `mailbox_runtime(mailbox_runtime: ArcShared<MailboxOf<R>>)` の設定 API を追加し、既存の `with_mailbox_factory` を段階的に廃止。
- `SchedulerSpawnContext`・`ActorSpawner`・`ActorSystemRunner` が共有ハンドルをどのタイミングで取得するかを明文化し、テストでカバー。

### フェーズ C: 周辺クレートとテストの反映
- `actor-std` / `actor-embedded` / `remote` 等のクレートで、`ActorRuntime` 依存箇所を一括で更新。
- E2E テスト (`core/tests.rs`, `cluster/tests.rs`) を `cargo test --workspace` で全体確認し、`RP2040` 向けクロスビルドが通ることを確認。

## 影響範囲メモ
- **core**: `dispatcher`, `cell`, `context`, `props`, `scheduler` が主。
- **remote**: `EndpointManager` がランタイム境界に触れるため、`MailboxOf` による再結線が必要。
- **cluster**: 生成コード (`cluster/generated/`) でのメールボックスファクトリ依存は削除予定。
- **message-derive**: メールボックス関連の trait bound を持たないため影響は軽微。
- **utils**: `ArcShared`, `AsyncCell` 等の共有基盤は再利用するのみ。

## テスト戦略
- ユニットテスト: `core/actor/tests.rs` で `mailbox_runtime()` を通じた取得が機能することを確認する新規テストを追加。
- 統合テスト: `actor-system` の起動パスで `mailbox_runtime_shared()` を複数スレッドから取得しても問題なく共有されることを検証。
- クロスビルドチェック: `cargo check -p nexus-actor-core-rs --target thumbv6m-none-eabi` と `--target thumbv8m.main-none-eabi` を両方走らせ、`no_std` 環境でも境界が破綻しないことを担保。
- Clippy: `cargo clippy --workspace --all-targets` で関連境界の冗長警告が出ないことを確認。

## リスクと対策
- **型爆発**: 複雑な `where` 句による読みにくさ → `MailboxOf<R>`, `MailboxProducerOf<R>` 等のローカルエイリアスを活用。
- **循環参照**: `ArcShared` の保持が循環にならないよう、`MailboxRuntime` 側は `WeakShared` を使って逆参照する。
- **API 互換性**: ランタイム外部公開 API（例: `actor-std`）は破壊的変更となるため、マイグレーションガイドを `CHANGELOG` 下書きに併記。
- **性能劣化**: `mailbox_runtime_shared()` の呼び出しコストを計測するため、`criterion` ベースのベンチマークを `actors/bench.rs` に準備。

## オープン課題
- `mailbox_runtime_shared()` を `Option` で返すか否か（シングルトン前提なら `ArcShared` を直接返すで良さそうだが、テスト差し替え時の柔軟性を要検討）。
- `MailboxRuntime` の関連型として `type MailboxRef = ArcShared<Self::Mailbox>` を導入する案の是非。
- `protoactor-go` における `SystemContext` 相当の構造を Rust でどこまで共通化するか。
- `no_std` 向けに `ArcShared` の代替を用意する必要があるかどうか（`alloc` 有効前提で良いか要判断）。

## 次のアクション候補
1. `core/runtime/actor_runtime.rs` で `mailbox_runtime_shared()` を導入し、テストを追加。
2. `ActorSystemBuilder` の API を `MailboxOf` 形式に合わせて刷新。
3. `docs/design/2025-10-15-actor-runtime-refactor-plan.md` に今回の方針を要約して追記。
