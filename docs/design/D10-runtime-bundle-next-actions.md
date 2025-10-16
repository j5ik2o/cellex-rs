# GenericActorRuntime / Runtime バンドル：次アクション

## 現状メモ
- `GenericActorRuntime` が runtime ハブとして稼働し、Tokio / Embassy 用拡張トレイトから scheduler・receive-timeout・metrics を差し替えられる。
- `ActorSystem::builder` が導入済みで、Runtime 既定値に対して `ActorSystemConfig` を重ねる構成に統一されている。

## 優先タスク
1. Runtime プリセット API（例: `GenericActorRuntime::host()`, `::embedded()`, `::remote()`) を整備し、ReceiveTimeout/Metrics/Event の既定セットを明示する。
2. `ActorSystemConfig` と Runtime 既定値の責務分離をドキュメント化し、優先順位（Runtime → Config）をコードサイドで保証する。
3. プラットフォーム別 ReceiveTimeoutDriver・MetricsSink（Prometheus / Defmt など）を実装し、Tokio / Embassy それぞれのプリセットに組み込む。
4. Embedded / Remote プロファイルでの統合テストと thumb ターゲットの CI チェックを整備する。
5. ドキュメントとサンプルコードを新しい Builder API に合わせて刷新する。

## 参考
- 旧メモは `docs/design/archive/2025-10-11-runtime-bundle-plan.md` を参照。
