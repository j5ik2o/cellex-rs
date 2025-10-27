# 提案: QueueMailboxCore を System/User 分離構造へ再設計する

## 背景
- 現状の `SystemMailboxQueue` は内部に `UserMailboxQueue` を抱き込み、System 予約レーンとユーザーメッセージ処理の二つの責務を同時に持っている。単一責務原則に反し、System 層の振る舞いを拡張・置換しづらい。
- ProtoActor / Pekko では mailbox core が system/user のキューを明確に分離した上で統合し、責務を切り出している。当プロジェクトでも同じ構造に揃えることで、将来の `LocalMailbox`・`WasmMailbox` 派生を容易にする。
- 既存の `QueueMailboxCore<Q, S>` は単一のキューに依存しており、System 専用レーンを差し替えるには `Q` 内でユーザーキューにアクセスする必要があるため抽象化が不十分。

## 変更内容
- リリースしていないので、破壊的変更を行ってください。
- `QueueMailboxCore` を `QueueMailboxCore<SQ, UQ, S>` へ拡張し、System キューと User キューを個別の実装で受け取る構造へ変更する。
- `SystemMailboxQueue` を System メッセージ専用キュー実装に再定義し、`UserMailboxQueue` への直接依存を解消する。
- Mailbox ビルダ・エイリアス・テストを新しい構造へ合わせて更新し、System レーン差し替えを容易にする。
- 既存の `UserMailboxQueue` の挙動と API は維持しつつ、System 層からの依存を取り除く。

## 成功条件
- `QueueMailboxCore` が System/User 双方のキューを独立して扱い、System 専用ロジックが `SystemMailboxQueue` 内に完結すること。
- 既存のテスト (`./scripts/ci-check.sh all`) がグリーンであり、新たな構造に合わせたテストが追加されること。
- System レーンを差し替える拡張ポイントが設計上明示され、将来的な Mailbox バリアントを阻害しないこと。
