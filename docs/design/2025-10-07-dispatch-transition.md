# dispatch_all 段階的非推奨ガイド (2025-10-07)

## 現状サマリ (2025-10-13)
- `PriorityScheduler::dispatch_all` は呼び出し時に `tracing::warn!` を出すのみで、API 自体は依然として公開されている。
- `run_until` / `dispatch_next` / `run_forever` への移行先は実装済みで、Tokio／Embassy いずれの環境でも利用可能。
- README は非推奨を明記済みだが、コード上の利用箇所やテストはまだ `dispatch_all` を参照している。

## 未解決課題
- [MUST] `PriorityScheduler::dispatch_all` と `RootContext::dispatch_all` に `#[deprecated(since = "0.2.0", note = "...")]` を付与し、コンパイル時に警告を出す。
- [MUST] テスト／サンプルから `dispatch_all` 呼び出しを排除し、`run_until` または `dispatch_next` への移行を完了させる。
- [SHOULD] `dispatch_all` を利用している外部 API（`actor-core` の sync ラッパ等）の代替手順をドキュメント化する。
- [SHOULD] 削除フェーズのマイルストーンを設定し、CHANGELOG にロードマップを追記する。

## 優先アクション
1. `#[deprecated]` 属性を追加し、ビルド警告が発生することを確認する。
2. `modules/actor-core/src/api/actor/tests.rs` など残存する呼び出しを `dispatch_next` ベースへ書き換える。
3. CHANGELOG と README に移行手順と削除予定バージョンを追記する。
