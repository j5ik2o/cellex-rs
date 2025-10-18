# Behavior Result Handler（分散エラー伝搬）

## サマリ（2025-10-18）
- Remote / Cluster 層の `FailureEvent` フローに対し、`ActorFailure` / `BehaviorFailure` のラップが維持されることを統合テストで確認。
  - `modules/remote-core/src/tests.rs` に `SampleBehaviorFailure` を用いた透過テストを追加。
  - `modules/cluster-core/src/tests.rs` に `ClusterBehaviorFailure` を用いた透過テストを追加。
- CI (`scripts/ci.sh`) の `std` セクションに `cellex-cluster-core-rs` テストを組み込み、分散経路の検証を自動化。

## 今後のフォローアップ
- DSL ドキュメントへのエラー設計追記、および `BehaviorFailure` カスタム実装サンプルの整備は別タスクで継続。

## 参照変更
- Remote: `modules/remote-core/src/tests.rs`
- Cluster: `modules/cluster-core/src/tests.rs`
- CI: `scripts/ci.sh`
