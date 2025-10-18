# Behavior Result Handler：次アクション

## 現状メモ
- `Behavior` は `Result<BehaviorDirective, ActorFailure>` を返す実装に統一され、`BehaviorFailure` トレイト／`ActorFailure` ラッパーが導入済み。
- Supervisor は `&dyn BehaviorFailure` を受け取る形へ刷新されている。

## 優先タスク
1. Remote / Cluster 層で `ActorFailure` と `BehaviorFailure` を透過させ、分散環境でのエラー伝搬を確認する。（済）
2. DSL ドキュメントにエラー設計と `?` 利用例を追記する。
3. `BehaviorFailure` のダウンキャスト例とカスタム実装のベストプラクティスをサンプルとして提供する。

## 参考
- 旧メモは `docs/design/archive/2025-10-13-behavior-result-handler.md` を参照。
