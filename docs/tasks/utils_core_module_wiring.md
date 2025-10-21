## utils-core を module_wiring_lint に準拠させる手順

1. `git status` で作業ツリーを確認し、`modules/utils-core` 以下に未コミット変更がないか確認する。必要なら作業用ブランチへ切り替える。
2. `makers ci-check -- dylint -n module-wiring-lint -m cellex-utils-core-rs` を実行し、Lint が指摘する違反箇所（`queue/storage.rs` と `stack/traits.rs`）を再確認してログを保存する。
3. `modules/utils-core/src/collections/queue/storage/queue_storage.rs` を編集し、`mod queue_alloc_impls` と `mod queue_std_impls` を削除する。代わりにファイル先頭へ `#[cfg(feature = "alloc")] use core::cell::RefCell;` と `#[cfg(all(feature = "alloc", feature = "std"))] use std::sync::Mutex;` を追加し、条件付き `impl` を直接トップレベルに定義する。トレイト本体や rustdoc コメントは保持する。
4. `modules/utils-core/src/collections/queue/storage.rs` でも同様に、`mod mpsc_alloc_impls` と `mod mpsc_std_impls` を削除し、条件付き `use` とトップレベル `impl` へ置き換える。`RingBufferStorage` トレイトと関連ドキュメントはそのまま維持する。
5. `modules/utils-core/src/collections/stack/traits/stack_storage.rs` を編集し、`mod alloc_impls` と `mod std_impls` を削除する。`#[cfg(feature = "alloc")] use core::cell::RefCell;` および `#[cfg(all(feature = "alloc", feature = "std"))] use std::sync::Mutex;` をファイル先頭へ追加し、対応する `impl` をトップレベルへ移動する。既存のトレイト定義とコメントは保持する。
6. 上記ファイルで不要になった `use super::QueueStorage;` や `use super::StackStorage;` などの内部モジュール専用インポートを削除し、`cargo check` 実行時に未使用警告が発生しないよう整理する。
7. `rg "QueueStorage" -n` と `rg "StackStorage" -n` で利用箇所を洗い出し、今回の変更で参照パスが変わっていないか確認する。ビルド時に解決不能なパスが出た場合は、該当ファイルを `crate::collections::queue::storage::QueueStorage` などの FQCN に更新する。
8. `makers fmt`（または `cargo +nightly fmt`）を実行し、 touched ファイルをプロジェクト標準フォーマットへ整形する。
9. `cargo check -p cellex-utils-core-rs` を実行して型エラーや未使用警告がないことを確認する。必要に応じて関連クレートでも `cargo check` を実行する。
10. `makers ci-check -- dylint -n module-wiring-lint -m cellex-utils-core-rs` を再実行し、module_wiring_lint がパスすることを確認する。追加で `cargo clippy --workspace --all-targets` を実行し、Lint 警告がないことを確認する。
11. プロジェクト規約に従い `./scripts/ci-check.sh all` を実行し、すべての検証が成功することを確認する。必要なら RP 向けターゲットへの `cargo check` もここで行う。
12. `git status` で変更ファイルを確認し、想定通りの差分のみか精査する。レビュー用に主な変更点と実行したコマンドをまとめ、後続のコミット／PR 作成に備える。
