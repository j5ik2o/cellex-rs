# ActorContext リファクタリング計画

## 背景
ActorContext は現状でメッセージ型 `M` をジェネリックとして受け取りつつ、実際のランタイム実装では `DynMessage` を中心に運用している。そのため Typed コンテキスト (`Context`) と Untyped コンテキスト (`ActorContext`) が混在し、責務の境界が不明瞭になっている。`ActorContext` の型パラメータを撤廃する前に、Typed ロジックが利用している箇所を `Context` へ移し、`ActorContext` を純粋な `DynMessage` ベースに整理する必要がある。

## ゴール
- `ActorContext` のメッセージ型パラメータ `M` を削除する。
- `ActorContext` の内部実装をすべて `DynMessage` に統一する。
- Typed ロジック (`Context`) は `ActorContext` をラップする形で維持し、ジェネリックなユーザーメッセージ型は `Context` 側に閉じ込める。
- ハンドラー定義・内部 API (`ActorHandlerFn`, `InternalProps` など) を `DynMessage` 固定へ段階的に移行する。

## ステップ
1. **Typed ハンドラーの洗い出しと移行方針策定**  
   - `ActorContext<'ctx, M, ...>` を直接引数に取る API／テストを調査する。  
   - 主な対象は `ActorHandlerFn` 定義、`InternalProps::new`、スケジューラ関連テストのヘルパーなど。  
   - Typed な処理が必要な箇所は `Context<'r, 'ctx, U, AR>` を介して扱うよう方針をまとめる。

   ### 調査結果（2025-10-18 時点）
   | 種別 | パス | 役割 | 備考 |
   | --- | --- | --- | --- |
   | 型エイリアス | `modules/actor-core/src/api/actor.rs` | `ActorHandlerFn` が `ActorContext<'ctx, M, ...>` を公開 API 経由で露出（2025-10-18 時点で `pub(crate)` 化済み） | Schedulers からのハンドラー登録口。Typed API が ActorContext に依存している根本原因であり、将来的に `Context` ベースへ置き換え予定。
   | 内部プロップ | `modules/actor-core/src/internal/actor/internal_props.rs` | `InternalProps::new` で `ActorContext<'ctx, M, ...>` を要求 | `Props` 生成時に Typed → Untyped の橋渡しを行う地点。ここを `Context` ベースに変換する必要。
   | ランタイム API | `modules/actor-core/src/api/actor/actor_context.rs` | `spawn_child` / `spawn_control_child` などが `F: FnMut(&mut ActorContext<'ctx, M, ...>, M)` を受け取る | ランタイム層で Typed クロージャを受け入れてしまっている。`Context` を受け取る API へ切り替えたい。
   | テスト補助 | `modules/actor-core/src/api/actor_scheduler/tests.rs` | テスト用 `handler_from_fn` が `ActorContext<'ctx, M, ...>` を直接要求 | テスト側も `Context` レイヤーをモックできるよう修正が必要。

   ### 論点メモ
   - `Context` は内部に `&mut ActorContext<'ctx, DynMessage, ...>` を保持しているが、型パラメータ付き `Supervisor<M>` を外部へ露出していない。Typed API で Supervisor 振る舞いを差し込むためには `SupervisorStrategyConfig` や `ActorAdapter` 側で完結する設計を維持しつつ、`Context` から必要な操作（`spawn_child`, `fail`, `watchers`, など）を提供しているか確認する必要がある。
   - 既存の `GuardianStrategy<M, MF>` などランタイム内部構造体は引き続きメッセージ型 `M` に依存している。②では API レイヤーから `ActorContext` の露出を減らすことに集中し、ランタイム内部の構造体は後続ステップ（③以降）でまとめて `DynMessage` 固定化を検討する。
   - `Context` へ置き換える際、`spawn_child` など親子間で異なるユーザーメッセージ型を扱うケース（例: 親: `ParentMessage`, 子: `ChildMessage`）が成立するよう、ジェネリック引数と `Props` の連携を再確認する必要がある。

2. **Typed API 呼び出しの `Context` 置き換え**  
   - スケジューラテストの `handler_from_fn` など、Typed メッセージを `ActorContext<'ctx, M, ...>` で扱っているコードを `Context` ベースへ書き換える。  
   - 必要に応じて `Context` から必要な機能を公開／調整し、Typed ロジックが `ActorContext` に直接触れないようにする。

   ### サブタスク案
   1. `ActorHandlerFn` を `Context` ベースのシグネチャに再定義し、スケジューラ API から `Context` を受けられるよう adapter 層を設計する。
   2. `InternalProps` / `Props` 連携を `Context` ベースに再実装する。`InternalProps` 内のハンドラ格納先を `DynMessage` handler ではなく `Context` handler を包むラッパーへ変更するか、Typed handler を `Context` で閉じ込めるラッパーを追加する。
   3. `ActorContext::spawn_child` など runtime API に存在する Typed クロージャ受け口を廃止し、Typed サイド（`Context`）に新しい `spawn_child_typed` API を導入するか、既存 `Context::spawn_child` を強化して `ActorContext` への依存を除去する。
      - ✅ `ActorContext::spawn_child` / `spawn_control_child` を削除し、`spawn_child_from_props` のみを内部利用とすることで Typed クロージャ経由の経路を閉じた。
   4. テストコード（特に `actor_scheduler/tests.rs`）を `Context` ベースのモック／適合関数に置き換え、Typed handler が `ActorContext` へアクセスしない形に整える。

3. **内部 API の `DynMessage` 固定化**  
   - `ActorHandlerFn`、`InternalProps`、`ChildSpawnSpec` など、内部的に `ActorContext<'ctx, M, ...>` を扱っている構造体・クロージャを `DynMessage` 固定へ更新する。  
   - `ActorContext::spawn_child` / `spawn_child_from_props` なども `DynMessage` 固定のシグネチャへ整理し、Typed レイヤーは `Context` で橋渡しする。

4. **`ActorContext` からジェネリック削除**  
   - 上記の移行が完了した段階で、`ActorContext` の定義からメッセージ型パラメータ `M` を除去し、フィールド・メソッドを `DynMessage` 前提に書き換える。  
   - `PhantomData` や型パラメータ依存のエイリアスを整理し、未使用コードを削除する。

5. **テスト・CI の実行**  
   - `cargo test --workspace` を実行して動作確認。  
   - `./scripts/ci-check.sh all` を実行してプロジェクト標準 CI チェックを通過させる。  
   - 必要ならば RP2040 / RP2350 向けターゲットへの `cargo check` も実施する。

## 留意事項
- type alias やラッパーで `DynMessage` 化を誤魔化さず、`ActorContext` 自体の定義を変更する。  
- `mod.rs` は利用せず 2018 モジュールスタイルを維持する。  
- 1 ファイルに複数構造体・トレイトを置かないという規約に従い、必要ならファイル分割を検討する。  
- 変更後も `Context` から必要な機能へアクセスできるようにするが、Typed/Untyped の境界は明確に保つ。  
- 既存実装（`protoactor-go`, `apache/pekko`, `docs/sources/nexus-actor-rs`）を参考にしつつ Rust イディオムへ落とし込む。
