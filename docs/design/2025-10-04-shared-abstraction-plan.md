# 所有モデル抽象化による std / embedded 両対応計画 (2025-10-04)

## 現状サマリ (2025-10-13)
- `cellex_utils_core_rs::sync::Shared` と `ArcShared` / `RcShared` を導入し、actor-core／actor-embedded の主要な共有所有権パスは抽象化済み。
- `MailboxRuntime` や `MessageSender` は `ThreadSafe` / `SingleThread` マーカーで同期境界を切り替えられるようになり、`embedded_rc` ビルドも維持できる状態。
- `ArcShared` は `target_has_atomic = "ptr"` が偽の場合に `Rc` へフォールバックする挙動が実装されており、単一スレッド環境での移植性が確保されている。

## 未解決課題
- [SHOULD] `StaticRef<T>` ベースの `embedded_static` フィーチャを実装し、完全静的メモリ構成でも `Shared` 抽象を利用できるようにする。
- [MUST] `alloc::sync::Arc` を直接公開している API（例: `api/actor/context.rs`, `behavior.rs`）を棚卸しし、`Shared` ラッパ経由の公開に統一する。
- [MUST] `std` / `embedded_rc` / `embedded_arc` 向けクロスビルドと単体テストを CI に追加し、バックエンド切替のリグレッションを自動検知する。
- [SHOULD] `Shared` 利用ガイドとベストプラクティスを README / CLAUDE.md に反映し、ユーザー向けドキュメントを更新する。

## 優先アクション
1. `cellex_utils_core_rs::sync` に `StaticRefShared` を追加し、例外処理と単体テストを整備する。
2. actor-core / actor-std の公開 API を調査し、`Arc` 露出箇所を `ArcShared` もしくは `Shared` トレイトに差し替える PR を準備する。
3. `cargo check -p nexus-actor-embedded-rs --no-default-features --features alloc,embedded_rc` など主要フィーチャ組み合わせを CI ワークフローに組み込み、結果を設計メモに反映する。
