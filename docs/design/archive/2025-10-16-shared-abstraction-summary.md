# Shared 抽象レイヤー：進捗と残タスク

## 完了済み
- `StaticRefShared` を追加し、静的参照向けのテストを整備（`modules/utils-core/src/sync/static_ref_shared.rs`）。
- `Shared` を従来どおり軽量な所有権トレイトとして維持しつつ、動的キャスト専用の `SharedDyn` を導入。`ArcShared` / `RcShared` / `StaticRefShared` が `SharedDyn` を実装済み。
- actor-core / actor-std の公開 API を確認し、直接 `Arc` / `Rc` を返す箇所が存在しないことを `rg "pub .*-> Arc<"` / `rg "pub .*-> Rc<"` で検証済み。

## 未完了タスク
- なし（2025-10-16 時点で対応完了）。

## アーカイブ移動
- すべてのタスクを完了したため、`docs/design/archive/2025-10-04-shared-abstraction-plan.md` に統合し、本メモはアーカイブへ移動可。

## 参考
- 旧計画メモは `docs/design/archive/2025-10-04-shared-abstraction-plan.md` を参照。
