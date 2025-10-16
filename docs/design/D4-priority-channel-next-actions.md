# Priority Channel 管理：次アクション

## 優先タスク
1. `PriorityEnvelope::from_system` の優先度テーブルを網羅的に検証するテストを追加する。現状は `modules/actor-core/src/tests.rs` の最小テストのみで、各 `SystemMessage` の優先度差分を確認できていない。
2. ユーザーメッセージ側から Control チャネルを選択する公式 API を検討する。
3. Remote / Cluster 経路で Control チャネル情報が保持されることを確認する統合テストを用意する。

## 参考
- 旧メモは `docs/design/archive/2025-10-07-priority-channel-table.md` を参照。
