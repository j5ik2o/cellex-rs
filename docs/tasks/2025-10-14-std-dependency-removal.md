# actor-core の std 依存整理メモ

目的: `cellex-actor-core-rs` のプロダクションコードから可能な限り `std` 依存を排除し、`no_std` 構成でも同等の機能を提供できるようにする。

## 現状の std 依存箇所

| ファイル | 対象行 | 役割 | 対応方針 |
| --- | --- | --- | --- |
| `modules/actor-core/src/runtime/scheduler/actor_cell.rs:8-9,205-244` | `catch_unwind` でハンドラを保護 | `std::panic::{catch_unwind, AssertUnwindSafe}` を使用 | - 2025-10-14 `unwind-supervision` feature を導入し、opt-in 時のみ `catch_unwind` を有効化。<br>- デフォルトでは `panic = "abort"` 前提の `Result` パスを使用。 |
| `modules/actor-core/src/runtime/system/internal_actor_system.rs` | 旧ブロッキングディスパッチ API | `futures::executor::block_on` 依存（削除済み） | 2025-10-14 に `blocking_dispatch_*` を削除し、async API のみ提供。追加対応不要。 |
| `modules/actor-core/src/api/actor/system.rs` | 旧 `ActorSystem::blocking_dispatch_*` | 内部ブロッキング API のラッパ（削除済み） | 2025-10-14 に削除済み。ドキュメント更新のみ追随。 |
| `modules/actor-core/src/runtime/scheduler/ready_queue_scheduler.rs` | 旧ブロッキングディスパッチ実装 | `futures::executor::block_on` / `tracing::warn!` 依存（削除済み） | 2025-10-14 に削除済み。非同期経路のみ維持。 |
| `modules/actor-core/src/api/supervision/escalation.rs:124-133` | エスカレーション時のログ出力 | `tracing::error!` を `std` 条件付きで使用 | - `tracing` に `std` が必要なため、`cfg(feature = "std")` ブロックを維持。<br>- `no_std` ではハンドラー / リスナー通知のみ行い、ログは省略。 |
| `modules/actor-core/src/extensions.rs:20-25,193-218` | Serializer 拡張 | JSON / Prost serializer の登録 | - これらクレートが `std` 前提のため、`std` 機能時のみ登録。<br>- `no_std` 用に Postcard (`cfg(feature = "postcard")`) を整備済み。 |

## 対応方針詳細

1. **パニック保護 (`catch_unwind`) の扱い**  
   - 設計方針は `docs/design/2025-10-14-actor-core-supervision.md` に集約。ここでは対応タスクの進行状況のみ管理する。

2. **ブロッキング API の整理**  
   - 2025-10-14 時点で `blocking_dispatch_*` 系メソッドをすべて削除済み。今後は async API を前提とし、同期実行が必要な場合は外部ランタイム側でラップする方針。
   - 必要であれば `no_std` 向けに軽量 executor を導入する検討を継続する。

3. **トレースログの非 `std` 対応**  
   - `tracing` 依存部分は `std` のみで有効化されるため、`no_std` ではハンドラ通知だけを行う現在の挙動を維持。
   - `no_std` 側でもログが必要な場合は、`log` crate + `defmt` 等の軽量バックエンド導入を検討。

4. **シリアライザ拡張**  
   - `std` 依存の JSON / Prost は現状維持。
   - `no_std` サポート強化のため、Postcard など軽量シリアライザを優先し、必要に応じて追加シリアライザの `feature` 化を行う。

4. **監査と CI**  
   - `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi` を CI に組み込み、`std` なしビルドが壊れていないか常時監視する。
   - `scripts/ci.sh embedded` 実行時に `DEFAULT_TOOLCHAIN`=stable で `no_std` チェックする体制を維持。

## 今後のタスク案

- `catch_unwind` を使ったパニック捕捉とエスカレーションが `std` 依存なので、非 `std` でも同様の失敗通知が可能か検討する。
- ログ出力を `tracing` 以外にも抽象化し、`no_std` 対応ログ（例えば `defmt`）を追加可能にする。
