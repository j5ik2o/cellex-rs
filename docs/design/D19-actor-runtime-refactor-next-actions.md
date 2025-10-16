# ActorRuntime リファクタ：次アクション

## 現状メモ
- `ActorSystem::builder` と `GenericActorRuntime` による統合が完了し、旧 `ActorSystemParts` からの移行も済んでいる。

## 優先タスク
1. Runtime 既定値と `ActorSystemConfig` の優先順位を明文化し、コードコメントとドキュメントで保証する。
2. プラットフォーム別 ReceiveTimeoutDriver／MetricsSink を整備し、Runtime プリセットに組み込む。
3. Embedded / Remote 向けの統合テストと thumb ターゲット CI を追加する。
4. README・サンプルを最新の Builder API に合わせて更新し、旧 API からの移行ガイドを提供する。

## 参考
- 旧メモは `docs/design/archive/2025-10-15-actor-runtime-refactor-plan.md` を参照。
