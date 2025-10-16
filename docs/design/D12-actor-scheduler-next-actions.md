# ActorScheduler 拡張：次アクション

## 現状メモ
- `SchedulerBuilder` に `with_scheduler_builder` / `with_scheduler_builder_shared` が導入され、Tokio / Embassy のビルダーもメールボックス型に合わせて更新済み。
- ReadyQueue ベースのスケジューラは tokio／embassy 両クレートで利用可能な状態。

## 優先タスク
1. Embassy 用スケジューラの統合テスト（シミュレーションも可）を追加し、`embedded_rc` / `embedded_arc` 構成での動作を自動検証する。
2. スケジューラごとのメトリクス収集・ベンチマークを整備し、Tokio / Embassy / Immediate の挙動差を可視化する。
3. スケジューラ差し替え手順をガイド化し、Best Practice を README／ドキュメントへ反映する。

## 参考
- 旧メモは `docs/design/archive/2025-10-12-actor-scheduler-options.md` を参照。
