# Actor Runtime 抽象リファクタリング計画 (2025-10-11)

## 現状の課題
- `ActorSystem::new` は `R: MailboxFactory` だけを受け取っており、Scheduler や ReceiveTimeout ドライバなど実行基盤の差し替えが想定されていない。
- `PriorityScheduler<R>` が `MailboxFactory` に強く依存しているため、Mailbox と Scheduler が実質的に結合している。
- Embedded / Tokio / Remote など異なるプラットフォーム向けに必要なコンポーネント（Scheduler、Timeout、EventListener、Metrics 等）をまとめて提供する仕組みが存在しない。

## ゴール
1. Mailbox と Scheduler を疎結合にし、プラットフォームごとに任意の組み合わせを選べるようにする。
2. `ActorSystem` へ渡すパラメータを「実行基盤バンドル (ActorRuntime)」として整理する。
3. `ReceiveTimeout` ドライバやイベント通知、メトリクスなど追加コンポーネントを段階的にバンドルへ移せるようにする。

## フェーズ別計画

### フェーズ 1: ランタイムバンドルの導入
- `ActorSystem::new(runtime: ActorRuntimeBundle)` のようにラップ構造体で受け取る。
- 現段階では `mailbox_factory` のみ保持し、既存呼び出しと互換性を維持する。
- バンドル構造体は `Default` 実装を持たせ、プラットフォームごとにビルダーを用意する（Tokio / Embassy / Local など）。

### フェーズ 2: Scheduler 抽象の切り出し
- `Scheduler` トレイト（spawn_actor / dispatch_next / run_forever）を定義し、`PriorityScheduler` を実装として登録。
- `ActorRuntimeBundle` に `scheduler: Arc<dyn Scheduler>` を格納し、MailboxFactory とは独立に差し替えられるようにする。
- MailboxFactory 側は必要な最小限のインターフェース（Queue / Signal）へ整理し、Scheduler からの依存を縮小する。

### フェーズ 3: 追加コンポーネントの統合
- ReceiveTimeout ドライバ、Escalation/Event リスナー、FailureHub などをバンドル内に移管。
- Host（std）、Embedded（no_std + alloc）、Remote 専用バンドルをそれぞれ定義し、必要なコンポーネントを組み合わせる。
- `ActorSystemBuilder` を導入し、アプリケーション側が個別コンポーネントを上書きできる設定 API を提供する。

## マイルストーン / TODO
- [ ] フェーズ 1 実装: `ActorRuntimeBundle` 追加、既存 API の 移行。
- [ ] フェーズ 2 設計レビュー: Scheduler トレイト定義と既存テストの影響調査。
- [ ] フェーズ 3 要件整理: Timeout・EventListener 等の利用箇所棚卸し。
- [ ] ドキュメント更新: README / ワークノートに新しい実行モデルのガイドを追記。

## 参考リンク
- `modules/actor-core/src/runtime`（Scheduler 実装）
- `modules/actor-embedded/src/embassy_dispatcher.rs`
- `docs/worknotes/2025-10-07-embassy-dispatcher.md`
