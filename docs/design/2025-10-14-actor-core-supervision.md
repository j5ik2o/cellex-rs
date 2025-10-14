# actor-core Panic / Supervision 設計メモ

最終更新: 2025-10-14

## 目的

`cellex-actor-core-rs` を `no_std` 環境でも利用できるように保ちつつ、パニック時のスーパービジョン挙動をどう設計するかをまとめる。

## 基本指針

1. **標準構成（デフォルト）**
   - `panic = "abort"` 前提。ハンドラは `Result` で失敗を返し、`panic!` は本当に致命的なバグ時のみ使用する。
   - 上位スーパーバイザは `Err(FailureInfo)` を受け取ってアクター差し替えを行う。パニックは即 abort するが、`panic_handler` で最小限のログや LED 表示を行う余地は残す。

2. **オプション構成（空間に余裕がある場合）**
   - Cargo feature 例: `unwind-supervision` を opt-in すると、`panic = "unwind"` と `std` 依存を有効化。
   - `std::panic::catch_unwind` を使用してパニックを捕捉し、`FailureInfo` に変換してガーディアンへ通知。アクター単位での再起動が可能になる。
   - 対象ターゲットやバイナリサイズ増加をドキュメント化し、CI でもこの構成を検証する。

3. **panic handler の役割**
   - `panic_handler` ではランタイム制御には戻らず、ログ出力や永続化（例: NVRAM、ウォッチドッグとの連携）等に限定する。
   - 高度な運用が必要な場合は、利用者が `panic_handler` 内で `PanicReporter` 相当の trait を呼び出し、再起動後に参照できる情報を蓄積する。

## 今後の改善案

- **現状の実装ギャップ**
  - 2025-10-14 時点の `modules/actor-core/src/runtime/scheduler/actor_cell.rs` では、`std` フィーチャ有効時に `std::panic::catch_unwind` でアクターハンドラを包み、パニックを `FailureInfo` としてガーディアンに通知している。
  - 設計方針で示した「`panic = "abort"` を基本とし、パニックは監督対象外とする」挙動と食い違っているため、該当ブロックの削除（もしくは `unwind-supervision` フィーチャ導入後に限定化）を行う。
- **Behaviors API の整理**
  - `Behavior::receive` / `Behaviors::receive` など “try なし” 系メソッドは `BehaviorDirective` のみを返し、失敗時は panic に依存している。`panic = "abort"` 方針と矛盾するため、まずは非推奨マークを付ける。
  - 既存の `try_receive` / `try_setup` 系を標準 API（`receive` / `setup`）へリネームし、すべて `Result<BehaviorDirective, ActorFailure>` 経路に収束させる。
  - `stateless` / `receive_message` など簡易ヘルパーは内部で `Ok(...)` を返すラッパに置き換え、panic 以外の失敗経路を確保する。
- `Result` ベースで失敗を伝播させる API をガイドラインとして整備し、末端アクターは “let it crash” を panic ではなく `Err` で表現できるようにする。
- `unwind-supervision` 有効時のコードサイズやターゲット制約を調査し、利用可能な MCUs を明示する。
- ログ出力を `tracing` 以外にも抽象化し、`no_std` 向けの `defmt` 等を統合できる余地を検討する。
