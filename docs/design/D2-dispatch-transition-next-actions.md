# dispatch_all 廃止の次アクション

## 優先タスク
1. `RootContext::dispatch_all` に `#[deprecated]` 属性を付与し、ビルド警告を発生させる。
2. テスト／サンプルコードから `dispatch_all` 呼び出しを排除し、`run_until` または `dispatch_next` に移行する。
3. CHANGELOG と README に移行手順と削除予定バージョンを追記する。

## ドキュメント更新
- `dispatch_all` 利用時の代替 API と注意点をガイド化する。

## 参考
- 旧メモは `docs/design/archive/2025-10-07-dispatch-transition.md` を参照。
