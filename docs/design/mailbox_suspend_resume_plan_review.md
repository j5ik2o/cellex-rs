# Suspend/Resume 実装計画レビュー

**レビュー実施日**: 2025-10-27  
**レビュー対象**: `docs/design/mailbox_suspend_resume_plan.md`  
**レビュアー**: Codex (GPT-5)

---

## サマリー

- 総合評価: 3.5 / 5.0 （基本方針は妥当だが、責務配置と実装詳細に未解決のリスクあり）
- 良い点: ゴールが明確、既存ドキュメントの参照が整理されている、ReadyQueue/メトリクス/テストを意識したステップ分解がある。
- 主な懸念: ADR-002 との整合、Mailbox レイヤでの Suspend 判定の実現性、no_std 向けの時間計測と同期プリミティブの選定が未確定。

---

## ブロッカー (Must Fix)

1. **ADR-002 との整合性が取れていない**  
   - 計画では `QueueMailboxCore` に `MailboxSuspension` を追加して状態管理を行うが、`docs/adr/2025-10-22-phase0-suspend-resume-responsibility.md` では Suspend/Resume の状態は `ActorCell` (MessageInvoker) が保持し、ReadyQueueCoordinator はステートレスに保つ方針が明示されている。  
   - Mailbox 側に状態を置くと、Invoker から `InvokeResult::Suspended` を返すフローが途切れ、ReadyQueue 連携が破綻するため、計画を ADR に合わせて ActorCell 側の状態管理へ修正する必要がある。

2. **QueueMailboxCore でユーザーメッセージを抑止する方法が不透明**  
   - 現行 `QueueMailboxCore` は `M: Element` の一般化されたキューであり、System/User の判定ロジックを持たない。提案通りに Suspend 中にユーザーメッセージをデキューしないようにするには、`PriorityEnvelope` を理解する新しいトレイト境界やキュー分割が必要になる。  
   - 計画にはそのための API 拡張方針が記載されていないため、実現性が担保されていない。ActorCell で `envelope.system_message()` を判定するアプローチ、またはキュー側の API 追加方針を明示することが不可欠。

3. **ReadyQueue とのハンドシェイク手順が欠落**  
   - 「Suspend 時に ReadyQueue への再登録を抑制する」とあるが、現行コードでは `InvokeResult::Suspended` を返し `ReadyQueueCoordinator::handle_invoke_result` を経由しなければ除外できない。Mailbox 内で通知抑止だけ行っても、既に登録済みのエントリが残り続ける。  
   - `dispatch_envelope` で `InvokeResult::Suspended` を返すフローと、Resume 時に `MailboxRegistry` 経由で `register_ready` を呼び戻すシーケンスを計画に追加すべき。

---

## 重要な改善提案 (Should Fix)

- **同期プリミティブと共有抽象の明確化**  
  - 計画中の `SharedMutex` はコードベースに存在しない名称。`cellex_utils_core_rs::sync::Shared` と `SpinSyncMutex` など既存プリミティブの組み合わせを使うのか、新規導入なのかを明示すること。  
  - 将来 `LocalMailbox` 実装にも適用できるよう、`MailboxSuspension*` ではなく `ActorSuspensionState` 等の命名と抽象化を検討すること。

- **no_std 向けの時間計測戦略**  
  - `Instant` は `thumbv6m-none-eabi` で利用不可。`SuspendMetricsClock` のような trait を用意し、`std` 環境では `Instant`、`no_std` 環境ではダミー記録もしくは `embedded-time` 等の抽象を差し込む設計を追加してほしい。  
  - 計測自体を feature gated にする場合も、API 表面をどうするかを文書化する。

- **バックプレッシャと enqueue 仕様の整理**  
  - Suspend 中に enqueue を許可するとバッファ無限化リスクがある。少なくとも Phase 0 では「Suspend 中は enqueue を許可するが ReadyQueue には再登録しない」「bounded queue overflow 時は Backpressure エラーを返す」などのポリシーを明文化すること。

- **MetricsEvent の拡張要件を具体化**  
  - `MetricsEvent::MailboxSuspended` 追加だけでなく、既存シンク（Prometheus/OpenTelemetry）の key/label 設計、サンプリングタイミング、再開時の duration 計測手順を記述する。  
  - `total_nanos` の集計と Exporter での取り扱いを明確にする。

---

## 補足提案 (Nice to Have)

- `MailboxSuspension` を導入する場合でも、テストや監視のために ActorCell から問い合わせ可能な API を提供（例: `fn suspension_snapshot(&self) -> MailboxSuspensionStats`）。
- `InvokeResult::Suspended { resume_on }` に紐づく Resume 条件（外部シグナル、タイムアウト等）を Phase 0 でどう扱うかを追記する。
- 文書末尾の完了条件に、ADR の整合性チェックや `cargo make coverage` によるベンチ確認を追加しておくと移行の抜け漏れ防止になる。

---

## テスト観点

- 単体テスト: `ActorCell::dispatch_envelope` に対する Suspend/Resume の基本ケース（システムメッセージ優先、二重 Suspend/Resume の冪等性、Suspend 中の Stop/Escalate 取り扱い）。
- 統合テスト: ReadyQueueCoordinator を含む suspend サイクル、bounded mailbox での overflow、複数アクター同時 Suspend の再開順序。既存テスト `test_typed_actor_stateful_behavior_with_system_message` の期待値更新も必須。
- ベンチ: suspend check 追加によるホットパス劣化が 5% 以内であることを `cargo bench actor_cell::dispatch` 系で確認する計画を明記する。

---

## 次のアクション

1. ADR-002 に基づいて責務セクションを全面的に書き換え、ActorCell 中心案を正式案にする（Mailbox 側はフラグ提供のみに留める）。
2. System/User 判定をどこで行うか、必要なら `QueueMailbox` に新しい API を追加する設計メモを追記する。  
3. no_std / embedded 向けの時間抽象と同期プリミティブの選択肢を決定し、計画へ反映する。  
4. テスト計画にエッジケースと既存テストの更新項目を列挙し、完了条件にも組み込む。

---

「MUST」を解消しない限り、実装に着手すべきではないと判断しました。上記の修正が計画に反映された段階で、改めてレビューしたいと思います。
