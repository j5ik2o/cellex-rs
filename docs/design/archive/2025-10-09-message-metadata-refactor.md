# MessageMetadata Typed 化計画メモ (2025-10-09)

## 現状サマリ (2025-10-13)
- `MessageSender<M, C>` / `InternalMessageSender<C>` による typed/untyped ブリッジは完了し、Ask 系 API は typed センダーのみで完結する。
- `MessageMetadata<C>` は `InternalMessageMetadata<C>` をラップする構造に整理され、`Context::respond` / `metadata.respond_with` から一元的に利用できる。
- `AskFuture` と responder 周りの糖衣 API は整備済みで、利用者が `InternalMessageSender` に直接触れるケースは解消されている。

## 未解決課題
- [MUST] `DynMessage` のサイズ増を避けるためのメタデータ格納方式（サイドテーブル案など）を実装し、メタデータ参照のコピーコストを削減する。
- [MUST] `ActorContext` / `Scheduler` に存在する `InternalMessageMetadata` 依存を棚卸しし、完全 typed 化または最小限のブリッジに落とし込む。
- [SHOULD] Ask/Respond のベンチマークを更新し、サイドテーブル案と現行実装の性能比較をドキュメント化する。
- [SHOULD] 公開ドキュメント（README・サンプル）を更新し、typed メタデータ API の使い方と落とし穴を整理する。

## 優先アクション
1. `MetadataTable`（仮称）の実装を試作し、enqueue/dequeue パスおよび panic 時のクリーンアップをテストする。
2. `ActorContext` で `InternalMessageMetadata` を直接扱っている箇所を調査し、抽象レイヤを追加するかどうかを決定する。
3. ベンチマーク (`cargo bench -p nexus-actor-core-rs --bench metadata_table`) を更新し、結果を設計メモに反映させたうえで採用可否を判断する。
