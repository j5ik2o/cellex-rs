# Shared 抽象レイヤー：未完了タスク

## 優先タスク
1. `cellex_utils_core_rs::sync` に `StaticRefShared` を追加し、完全静的メモリ構成向けのユニットテストを整備する。
2. actor-core / actor-std の公開 API を棚卸しし、直接 `Arc` を返している箇所を `ArcShared` もしくは `Shared` トレイトを通す実装へ置き換える。
3. `std` / `embedded_rc` / `embedded_arc` 向けクロスビルドと主要テストを CI ワークフローに追加する。

## ドキュメント整備
- `Shared` 抽象の利用ガイドとベストプラクティスを README / CLAUDE.md に追記する。

## 参考
- 旧計画メモは `docs/design/archive/2025-10-04-shared-abstraction-plan.md` を参照。
