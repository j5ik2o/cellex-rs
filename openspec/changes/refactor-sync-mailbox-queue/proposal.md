# 提案: SyncMailboxQueue を UserMailboxQueue へ改名する

## 背景
- 現在の `SyncMailboxQueue` という名称は「同期的な実装であること」を強調しており、実際の責務である「ユーザーメッセージ用キュー」である点が伝わりづらい。`SystemMailboxQueue` が導入されたことで、この非対称性が一層目立っている。
- ProtoActor / Pekko では user/system キューが対称的に扱われており、当プロジェクトでも同じ概念を分かりやすく表現したい。
- 責務を明確にすることで、今後の mailbox 派生（ローカル向け、Wasm 向けなど）を追加する際の混乱を防ぎ、System レーンとの意図しない結合を避けられる。

## 変更内容
- コアのキュー実装 `SyncMailboxQueue` と関連する型エイリアスを `UserMailboxQueue` 系の命名へ置き換える。
- `SyncMailbox` / `SyncMailboxProducer` などのエイリアスやファクトリも、ユーザー向けであることが分かる名前へ整理する（必要に応じて公開 API の互換性を検討）。
- ドキュメント・テスト・Spec を更新し、「ユーザーメッセージ専用であり System レーンの責務を持たない」ことを明記する。
- 動作面では変更を加えず、あくまで命名と責務の明確化に留める。

## 成功条件
- コード上の `SyncMailboxQueue` 参照が全て新しい命名へ置き換わり、ビルドが通ること。
- Spec 上で、ユーザーキューが独立した責務を持つこと・System 側は別レイヤであることが明示されること。
- `./scripts/ci-check.sh all` を含むテストがすべて成功し、命名変更による回帰が発生しないこと。
