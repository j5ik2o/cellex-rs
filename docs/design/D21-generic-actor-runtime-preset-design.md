# D21: GenericActorRuntime プリセット/並列度選択 API 設計

- 作成日: 2025-10-21
- 更新日: 2025-10-21
- 作成者: Codex (GPT-5)
- 関連ドキュメント: [D14-mailbox-runtime-next-actions](./D14-mailbox-runtime-next-actions.md), [D10-runtime-bundle-next-actions](./D10-runtime-bundle-next-actions.md), [D19-actor-runtime-refactor-next-actions](./D19-actor-runtime-refactor-next-actions.md)

## 概要

`GenericActorRuntime` は actor-core 内で mailbox factory と周辺設定を束ねる汎用バンドルであり、Tokio や Embassy といった環境固有の処理は周辺クレート側で定義される。したがって core から環境固有実装を生成することはできず、上位層（例: `TokioActorRuntime`, `TokioActorSystem`）が `GenericActorRuntime` と `ActorSystem` を内包したハイレベル API を提供する必要がある。本ドキュメントでは、そうしたレイヤー分離を前提にプリセット構築の責務を整理し、SingleThread 構成での不要な `Send + Sync` 境界も合わせて検討する。

## 背景

- 既存のプリセットは `actor-std::tokio_actor_runtime` や `actor-embedded::embassy_actor_runtime` といった関数に分散しており、統一された設定オブジェクトがない。
- `actor-core` は `actor-std`・`actor-embedded` に依存できないため、`GenericActorRuntime` 側から Tokio 等を参照する API は設計上定義できない。
- `MailboxFactory::Concurrency` が `SingleThread` の場合であっても、`SharedBound` に由来する `Send + Sync` 制約が残っており、組み込み用途で不要な境界が生じている。

## ゴール

1. 環境クレート側で「プリセット（例: Tokio 用）」を表す構造体/ビルダーを用意し、`GenericActorRuntime<MF>` を組み立てて返す導線を整備する。さらに、そのプリセットを内部で利用する高レベル API (`TokioActorRuntime`, `TokioActorSystem`) を実装し、利用者はこのファサード経由で起動・停止まで完結できるようにする（D20, Archive/2025-10-15 参照）。
2. 利用者がプリセット経由で並列度（`ThreadSafe` / `SingleThread`）を選択できるようにし、SingleThread 選択時は `Send + Sync` 境界を外せる設計にする。
3. Remote プリセットについては将来実装に向けたプレースホルダ（`todo!()` もしくは `Err(NotImplemented)`）を設け、現時点で利用不可であることを明示する。

## 非ゴール

- Remote mailbox runtime 実装そのもの。
- `ActorSystem::builder` など既存ビルダー API の大幅な仕様変更。
- Backpressure や `MailboxOptions` 拡張（D14 の別タスクで対応）。

## 現状と課題

### 既存 API の整理

| 項目 | 状態 |
| ---- | ---- |
| `GenericActorRuntime::new` | `MailboxFactory` を直接受け取り、scheduler 等はメソッドチェーンで個別設定 |
| 環境ごとのプリセット | `actor-std` / `actor-embedded` が個別関数（`tokio_actor_runtime()` 等）で返却。設定項目はハードコード |
| 並列度切替 | `MailboxFactory::Concurrency` で表現するが、呼び出し側が能動的に選択する仕組みはない |
| `Send + Sync` 境界 | `SharedBound` により pointer-atomic ターゲットでは常に `Send + Sync` が要求され、SingleThread 構成でも緩和されない |

### 課題

1. **プリセット構築の分散**: 設定が関数実装に埋め込まれており、Tokio/Embassy で共通化しづらい。
2. **並列度指定の欠落**: compile-time feature 切替以外に SingleThread を選ぶ手段がない。
3. **境界の過剰厳格化**: `QueueMailboxProducer` などで `Send + Sync` が必須のままになっている。

## 提案する設計

### 1. プリセット構築の責務分離とハイレベルレイヤー

- `actor-std` に `TokioActorRuntimePreset`（仮称）構造体と `TokioActorRuntimePresetBuilder` を追加。
  - フィールド例: `mailbox_runtime: TokioMailboxRuntime`, `scheduler: ActorSchedulerHandleBuilder<_>`, `receive_timeout: Option<...>`, `metrics: Option<MetricsSinkShared>`, `concurrency: TokioConcurrencyMode`。
  - `into_runtime(self) -> GenericActorRuntime<TokioMailboxRuntime>` を実装し、内部で `GenericActorRuntime::new(self.mailbox_runtime)` に設定を適用。
  - 既存の `tokio_actor_runtime()` は `TokioActorRuntimePreset::default().into_runtime()` の薄いラッパへ移行。
- さらにプリセットを利用するハイレベル構造体として `TokioActorRuntime`（`GenericActorRuntime` を内包し、Tokio 固有設定を固定化）と `TokioActorSystem`（`ActorSystem` + `TokioSystemHandle` を内包）を `actor-std` に追加する。
  - `TokioActorRuntime` は `TokioActorRuntimePreset` を内部的に用いて初期化し、公共 API としては `TokioActorRuntime::new(options)` などを提供。
  - `TokioActorSystem::new(props, name, options)` は Guardian Props を受け取り、内部で `TokioActorRuntime` + `ActorSystem::builder` をひとまとめに起動する。設計方針は [D20](./D20-actor-system-entrypoint-next-actions.md) と archive/2025-10-15 の計画に従う。
- `actor-embedded` も同様に `EmbeddedActorRuntimePreset` を提供し、`LocalMailboxRuntime` を用いて `GenericActorRuntime<LocalMailboxRuntime>` を生成。
- `EmbeddedActorSystem`（仮称）についても必要性を評価し、Tokio と同じレイヤリングで提供する。
- `remote-core` は `RemoteActorRuntimePreset::new()` を仮置きし、現時点では `Err(RemotePresetError::NotImplemented)` を返す。
- `GenericActorRuntime` 自体は環境固有型を知らず、プリセット構造体にも依存しない。受け取った設定を適用するためのメソッド群（`with_scheduler_builder` など）は既存のまま利用する。

### 2. 並列度設定と API

- 各プリセット構造体に `concurrency` フィールド（`enum { ThreadSafe, SingleThread }`）を持たせる。
- Tokio 向け SingleThread 選択時には、`TokioMailboxRuntime` とは別に `TokioLocalMailboxRuntime`（仮称）を導入し、`MailboxFactory::Concurrency = SingleThread` を返す。
- Embedded 向けは既存の feature (`embedded_rc` / `embedded_arc`) と `concurrency` フィールドを同期させる。明示的な指定が無い場合は feature から自動推論するが、オプションを尊重する優先順位を設ける。

### 3. `Send + Sync` 緩和に向けた core 修正

- `actor-core` に `ConcurrencyProfile` トレイトを追加し、`ThreadSafe` / `SingleThread` で `const REQUIRES_SEND_SYNC` を切り替え。
- `QueueMailbox` / `QueueMailboxProducer` に `PhantomData<C>` を導入し、`C: MailboxConcurrency + ConcurrencyProfile` を伝播させる。
- pointer-atomic ターゲットでの unsafe impl を `REQUIRES_SEND_SYNC` が `true` の場合にのみ提供するよう定義し、SingleThread 選択時には `Send` / `Sync` を実装しない。
- 既存コードの呼び出し側に影響が出ないよう type alias を整備し、ジェネリクスのノイズを最小限に保つ。

### 4. 互換 API の維持

- 現行の `TokioActorRuntime` 型 alias・`tokio_actor_runtime()` 関数は公開インターフェースとして維持。
- 内部実装のみプリセット構造体経由へ置き換え、既存ユーザーコードへの破壊的変更を避ける。
- Embedded も同様に `embassy_actor_runtime()` などの公開 API を維持。

## 実装計画

1. **core 側準備**
   - `ConcurrencyProfile` と `PhantomData` 化による `Send + Sync` 条件分岐を実装。
   - `QueueMailbox` 系の型 alias を整理し、既存呼び出し箇所を更新。
2. **Tokio プリセット導入**
   - `TokioActorRuntimePreset` とビルダーを追加。
   - `tokio_actor_runtime()` / `TokioActorRuntimeExt` をプリセット経由に変更。
   - `TokioLocalMailboxRuntime` を実装し、SingleThread モードをサポート。
3. **Embedded プリセット導入**
   - `EmbeddedActorRuntimePreset` を追加し、feature/オプションの同期ロジックを実装。
   - `embassy_actor_runtime()` 等をプリセット経由に変更。
4. **Remote プレースホルダ**
   - `RemoteActorRuntimePreset` を追加し、`Err(RemotePresetError::NotImplemented)` を返す導線を提供。
5. **テスト/検証**
   - `cargo test --workspace` を ThreadSafe / SingleThread 両構成で実行。
   - `cargo check -p cellex-actor-embedded-rs --target thumbv6m-none-eabi` 等で SingleThread モードを検証。
   - 型境界の単体テスト（`static_assertions`）で `Send` の有無を確認。

## 検証方針

- プリセット構造体のデフォルト値が既存挙動と一致することを単体テストで確認。
- `QueueMailboxProducer` の `Send` 実装が SingleThread 選択時にコンパイルされないことを型テストで担保。
- `scripts/ci-check.sh all` にクロスチェック（Tokio/Embedded）を追加し、CI で回帰を検出。

## リスクとフォローアップ

- `QueueMailbox` のジェネリクス増加に伴う型推論コスト増。必要に応じて type alias や builder メソッドで隠蔽する。
- `TokioLocalMailboxRuntime` の実装負担。Tokio current-thread で動作させるための signal/queue 実装が必要となる。
- Remote プリセットが未実装のまま公開される点は、`Err(RemotePresetError::NotImplemented)` で明示的に利用不可とし、リリースノートで周知する。
