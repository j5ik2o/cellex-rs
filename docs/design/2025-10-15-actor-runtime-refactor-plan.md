# ActorRuntime 再設計プラン

## 背景
- 旧実装の `ActorSystemParts::new(RuntimeEnv, Spawner, Timer, FailureEventHub)` は、利用者に実行環境の内部構成を意識させており、責務分離が不明瞭だった。
- `RuntimeEnv` の名称は「実行環境全体」を示唆する一方で、実態はメールボックス・シグナル処理のみを担当しており、`Spawner` や `Timer` が別引数になっている。
- ガイドラインとして「Shared 抽象モデル」を志向しており、`std` / `no_std` を問わず一貫した API で構築できる形が望ましい。

## 課題
1. `RuntimeEnv` によって実際に得られる機能と名称が一致しておらず、MECE ではない構成になっている。
2. 実行環境を差し替える際に複数コンポーネントを明示的に指定する必要があり、利用者体験が悪い。
3. `std` 用と `no_std` 用でどう構成を変えるかが明文化されておらず、設計意図の読み解きが困難。

## 目標
- 利用者が単一の `ActorRuntime` 実装を差し込むだけで、アクターシステムの脚まわり（Spawner/Timer/FailureEventHub/MailboxRuntime）が整う状態にする。
- 命名と抽象が一致するように再構成し、ドキュメントとコードから意図が読み取れるようにする。
- `std` / `no_std` それぞれに対してデフォルト実装を提供しつつ、細かな差し替えも可能にする。

## 推奨アーキテクチャ
- トレイト `ActorRuntime` を定義し、以下を公開アクセサで提供する：
  - `Spawner`: 非同期タスク起動 (`spawn`)
  - `Timer`: 遅延実行 (`sleep`)
  - `FailureHub`: 失敗イベント配信 (`emit` など)
  - `MailboxRuntime`: メールボックス生成とシグナル管理
- 汎用実装 `GenericActorRuntime<Spawner, Timer, FailureHub, MailboxRuntime>` を用意し、各構成要素を型パラメータで受ける。
- `StdActorRuntime`, `NoStdActorRuntime`, `EmbeddedActorRuntime` などのプリセット構造体を提供し、それぞれ `GenericActorRuntime` を内部で利用する。

## API 変更案
- `ActorSystemBuilder::with_actor_runtime(runtime: impl ActorRuntime)` 形式に変更し、`ActorSystemParts` の公開 API を段階的に廃止する。（`ActorSystem::new_with_runtime_and_event_stream` の導入により、`ActorSystemParts` は 2025-10-15 時点でリポジトリから削除済み）
- 必要であれば `StdActorRuntime::builder()` を提供し、`Timer` などの細部差し替えを支援する。
- `ActorSystemHandles` は `ActorRuntime` から必要なハンドルを参照する仕組みに移行し、利用者に追加コンポーネントを返さない。

## 移行ステップ
1. `ActorRuntime` トレイトと `GenericActorRuntime` を導入し、既存実装をラップする形で移植する。
2. `StdActorRuntime` と `NoStdActorRuntime`（仮称）を整備し、テストコードを新 API に差し替える。
3. `ActorSystemParts` を内部実装に置き換え、ビルダ API を `with_actor_runtime` へ移行する。（`ActorSystemParts` は既に削除済みのため、この項目は完了）
4. ドキュメント・サンプルを更新し、利用者に新 API への移行手順を提示する。

## 影響評価
- **互換性**: 破壊的変更（`ActorSystemParts` の直接利用を想定しているコードが影響）。
- **テスト**: `actor-std/tests/runtime_driver.rs` などを含む既存テストの書き換えが必要。
- **no_std 対応**: `NoStdActorRuntime` を整備することで明示的にサポートしやすくなる。

## オープン課題
- `FailureHub` のインターフェース命名（`FailureTelemetry` との整合性）。
- `GenericActorRuntime` の所有権ポリシー（`Arc` を使うか、参照を返すか）。
- `ActorRuntime` 経由でのメトリクス導入をどう位置づけるか（ログ以外の拡張ポイント）。

## 次アクション
1. `ActorRuntime` トレイト案と `GenericActorRuntime` の型スケッチを作成し、レビューを依頼する。
2. `StdActorRuntime` の初期実装を起こし、既存テストを `with_actor_runtime(StdActorRuntime::default())` に切り替える PoC を作成する。
