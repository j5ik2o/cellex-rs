# DeadLetter / Process Registry 再設計：次アクション

## 目的
- Remote/Cluster を含む分散環境で、未達メッセージや未登録 PID 向けメッセージを失わずに観測できる DeadLetter 経路を整備する。
- PID 解決 (`ProcessRegistry`) を再設計し、Control チャネル情報を保持したまま `PriorityEnvelope` を配信できるようにする。
- エラー伝搬 (`ActorFailure`/`BehaviorFailure`) と DeadLetter 通知を組み合わせ、監視・テレメトリに統合する。

## 現状課題
- PID 解決の仕組みが未実装であり、Remote/Cluster 経路での Actor 参照解決が確立していない。
- DeadLetter の受け口がなく、未達メッセージを観測・再送する手段が存在しない。
- Control チャネル付きの `PriorityEnvelope` が DeadLetter 経由で保持される保証がない。
- Remote/Cluster 統合テストが DeadLetter シナリオをカバーしていない。

## 優先タスク
1. **ProcessRegistry 設計**
   - PID ↔ `PriorityActorRef` の双方向登録 API
   - 名前一意性 (Cluster シャーディングを考慮) と再登録ポリシー
   - Remote との PID 名前空間整合 (例: `actor://cluster/node/id` 形式)
2. **DeadLetter Mailbox 設計**
   - `DeadLetter` メッセージ型と Control チャネル保持方針
   - DeadLetter サブスクライブ API（メトリクス／ログ）
   - Telemetry / FailureHub との連携ポイント
3. **配信フロー更新**
   - PID 解決失敗・停止済み PID に対する DeadLetter ルートの追加
   - Remote/Cluster 経路での未解決 PID 処理
4. **統合テスト計画**
   - ローカル: 未登録 PID、停止済み PID、即停止 Actors
   - Remote: ネットワーク切断 / PID 未登録 / Control チャネル保持
   - Cluster: PID 移動・再配置レース時の DeadLetter 化

## 参照
- D4: Priority チャネル整合性
- D15: 分散エラー伝搬確認
- D7: ProcessRegistry 基本機能パリティ
