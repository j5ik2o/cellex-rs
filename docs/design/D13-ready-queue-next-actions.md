# ReadyQueue スケジューラ改善：次のアクション

## 優先タスク

1. ReadyQueue ワーカー構成のチューニング方針を固める  
   - 使用するキュー実装（`mpsc` / lock-free など）  
   - 1 ワーカーあたりの処理スループットと `yield_now` タイミング  
   - 主要メトリクス項目（滞留長・再スケジュール発生回数など）の確定
2. Spawn ミドルウェアとの統合方式を設計し、`ChildNaming` と連携した公開 API を整理する  
   - 生成時に ReadyQueue / Mailbox をどう初期化するか  
   - 名前付きスポーン時のエラー整理とテレメトリ連携
3. ReadyQueue の観測ポイントを増強する  
   - ワーカー数・キュー滞留長・処理レイテンシの計測を追加  
   - `tracing` / メトリクス用エクスポータの更新計画を用意

## 備考
- 元の設計メモは `docs/design/archive/2025-10-15-scheduler-ready-queue.md` に移動済み。
- 上記が完了したら、Tokio / Embassy 双方での評価結果をまとめてドキュメント化する。
