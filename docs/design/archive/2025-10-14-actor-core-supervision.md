# actor-core Panic / Supervision 設計メモ

最終更新: 2025-10-14（CI 経由確認済み）

## 目的

`cellex-actor-core-rs` を `no_std` 環境でも利用できるように保ちつつ、パニック時のスーパービジョン挙動をどう設計するかをまとめる。

## 基本指針

1. **標準構成（デフォルト）**
   - `panic = "abort"` 前提。ハンドラは `Result` で失敗を返し、`panic!` は本当に致命的なバグ時のみ使用する。
   - 上位スーパーバイザは `Err(FailureInfo)` を受け取ってアクター差し替えを行う。パニックは即 abort するが、`panic_handler` で最小限のログや LED 表示を行う余地は残す。

2. **オプション構成（空間に余裕がある場合）**
   - `panic = "unwind"` を許容できるターゲット向けに、`catch_unwind` ベースの経路を選択制とする（Cargo feature: `unwind-supervision`）。
   - デフォルト（`alloc` のみ / `std` のみ）では `catch_unwind` をリンクしない。`--features std,unwind-supervision` を指定した場合のみ `std::panic::catch_unwind` と監督ハンドリングが動作する。
- CI (`scripts/ci-check.sh std`) でも `std,unwind-supervision` 付きで `cellex-actor-core-rs` のテストを実行し、オプション経路が回帰しないようにしている。

3. **panic handler の役割**
   - `panic_handler` ではランタイム制御には戻らず、ログ出力や永続化（例: NVRAM、ウォッチドッグとの連携）等に限定する。
   - 高度な運用が必要な場合は、利用者が `panic_handler` 内で `PanicReporter` 相当の trait を呼び出し、再起動後に参照できる情報を蓄積する。

## 今後の改善案

- **実装状況**
  - `modules/actor-core/src/runtime/scheduler/actor_cell.rs` の `catch_unwind` ブロックを `#[cfg(feature = "unwind-supervision")]` でガード済み。既定では Result ベースの経路のみを使用し、panic は `panic_handler` で処理する。
  - panic 捕捉に依存するテスト (`runtime/scheduler/tests.rs` の一部) も `#[cfg(all(feature = "std", feature = "unwind-supervision"))]` で包み、`std` 単独ビルド／`no_std` ビルドではコンパイル対象にならないよう整理した。
- CI でも `scripts/ci-check.sh std` が `--features std,unwind-supervision` を付与して実行されるため、オプション経路の回帰検知が可能になった。
- **Behaviors API の整理**
  - `Behavior::receive` / `Behaviors::receive` は Result を返す実装へ統一済み。panic 依存を避けるため、すべてのハンドラは `Result<BehaviorDirective, ActorFailure>` を返す。
  - 旧 `try_*` 系 API (`try_receive` / `try_receive_message` / `try_setup`) は完全に削除した。既存コードは Result 返却の `receive` / `setup` を利用する。
  - `stateless` / `receive_message` など簡易ヘルパーは内部で `Ok(...)` を返すラッパに置き換え、panic 以外の失敗経路を確保する。
- `Result` ベースで失敗を伝播させる API をガイドラインとして整備し、末端アクターは “let it crash” を panic ではなく `Err` で表現できるようにする。
- `unwind-supervision` 有効時のコードサイズやターゲット制約を調査し、利用可能な MCUs を明示する。
- ログ出力を `tracing` 以外にも抽象化し、`no_std` 向けの `defmt` 等を統合できる余地を検討する。
