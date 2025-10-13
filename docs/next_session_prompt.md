# 継続作業プロンプト（次セッション引き継ぎ用）

## 現状
- 旧 `ActorRuntime` トレイトがメールボックス工場の責務と高レベル環境機能を同居させており、依存方向の整理が進んでいない。
- `RuntimeEnvCore<R>` / `RuntimeEnv<R>` は既存どおり `R` を保持し、型 `R` からメールボックス生成ロジックを委譲する構造になっている。
- `ActorSystem<U, R, Strat>` は `R: ActorRuntime` を要求し、`RuntimeEnv` を直接差し込む前提で構成されている。
- 既存テストはグリーン（直近は `cargo test --workspace` 済）。

## 目的
- メールボックス工場インターフェイスと高レベル環境インターフェイスを分離し、責務を明確化する。
- `RuntimeEnv` はこれまでどおり `MailboxRuntime`（旧 `ActorRuntime`）を保持しつつ、新しい `ActorRuntime` を実装して高レベル API を提供できるようにする。
- `ActorSystem` / `InternalActorSystem` などの利用側は、新 `ActorRuntime` を受け取り `RuntimeEnv` を差し込める設計に移行する。
- 将来的な `TokioActorRuntime` / `EmbassyActorRuntime` などの facade 追加を容易にする。

## 方針
0. **型アノテーションの事前整理**
   - `type RuntimeParam<R> = RuntimeEnv<R>` など `RuntimeEnv` 固定の別名を廃止し、必要なら `R` そのものを使う。
   - 実装／テストコードで `RuntimeEnv<...>` を明示しているローカル変数・関数戻り値は、型推論に任せられるところから削除。
   - 明示が必要な箇所は最小限の境界（`impl MailboxRuntime` など）に寄せる。`ActorSystem` などのローカル変数注釈は `ActorSystem<_, _, _>` への部分推論、あるいは注釈ごと削除しておくとステップ4後のコンパイルエラーを防げる。
   - `Context` / `ActorRef` / `Props` / `RootContext` / `MessageEnvelope` から `RuntimeParam` 別名を撤去し、必要箇所は `RuntimeEnv<R>` を直接参照する形に整理（2025-10-13 完了）。
   - `spawn_actor_with_counter_extension` などテスト補助関数もジェネリック化して `TestMailboxRuntime` 固定を解消済み。
   - 静的領域で具体型が必須なケース（例: `StaticCell<ActorSystem<u32, LocalMailboxRuntime, _>>`）は現状維持。今後の facade 導入時に個別対応すること。
   - 変更後は `cargo check -p cellex-actor-core-rs` を小刻みに回して検証。

1. **トレイトのリネーム** (AIでやると遅いのでIDEでリファクタリング済み)
   - `modules/actor-core/src/runtime/mailbox/traits.rs` にある現行 `trait ActorRuntime` を `trait MailboxRuntime` にリネーム。
   - 依存ファイルの `use` とジェネリクス境界を総置換。
   - テスト／ドキュメント内の呼称も `MailboxRuntime` に揃える。

2. **新しい `ActorRuntime` トレイトを定義**
   - 旧 `RuntimeEnv` が外部へ提供している高レベルメソッド（`mailbox_runtime`、`scheduler_builder`、`receive_timeout_*`、`metrics_sink` など）を洗い出し、新`trait ActorRuntime: MailboxRuntime` として定義。
   - トレイトは `RuntimeEnv` と将来の facade 型が実装できるように、既存 API をそのままインターフェイス化する。

3. **`RuntimeEnv` に新トレイトを実装**
   - `impl<R: MailboxRuntime + Clone + 'static> ActorRuntime for RuntimeEnv<R>` を追加し、既存メソッドを委譲。
   - `ActorRuntime` 実装内で `RuntimeEnv` の状態管理（タイムアウト、メトリクス等）をそのまま活かす。
   - `RuntimeEnvCore`/`RuntimeEnv` の内部構造は変更しない。

4. **利用側を新トレイトに更新**
   - `ActorSystem<U, R, Strat>`、`ActorSystemConfig<R>`、`InternalActorSystem<M, R, Strat>` などで `R: ActorRuntime` を要求するよう調整。
   - `ReceiveTimeoutFactoryShared<DynMessage, R>` など型引数の更新を忘れずに行う。
   - 既存の `type RuntimeParam<R>` など `RuntimeEnv` 固有型エイリアスがあれば廃止／整理。

5. **ドキュメントとコメントの同期**
   - `docs` 配下の旧用語（`MailboxRuntime` → `ActorRuntime`）を新しい区分に合わせて更新。
   - 仕様メモに「ユーザは `MailboxRuntime` を実装しつつ facade を選択できる」設計意図を明記。

6. **将来拡張の下準備**
   - `TokioActorRuntime` や `EmbassyActorRuntime` のような facade 型を追加する際の雛形を検討（`RuntimeEnv` を内包しつつ `ActorRuntime` を実装する構造）。
   - 必要ならテストサポート用のダミー facade を追加して API 適合性を確認。

## 受け入れ条件
- `cargo test --workspace` が成功すること。
- `cargo clippy`は不要。実装優先です。
- 可能なら RP2040/RP2350 向けクロスチェック：
  - `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi`
  - `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi`

## 参考ファイル
- `modules/actor-core/src/runtime/mailbox/traits.rs`
- `modules/actor-core/src/api/actor/system.rs`
- `modules/actor-core/src/api/actor/{context.rs, actor_ref.rs, props.rs, root_context.rs}`
- `modules/actor-core/src/runtime/system/internal_actor_system.rs`
- `modules/actor-core/src/api/messaging/message_envelope.rs`

## 注意
- mod.rs 禁止（2018 モジュール規則）。
- rustdoc (`///`, `//!`) は英語、それ以外のコメントは日本語。
- 破壊的変更は許容。ただし段階的にコンパイルを維持しながら進める。

## 実行コマンド例
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo +nightly fmt`
