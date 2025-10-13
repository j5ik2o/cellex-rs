# Behavior Result Handler 拡張案 (2025-10-13)

## 背景
- 現在の `Behavior` は `FnMut(Context, Msg) -> BehaviorDirective` を返すインターフェースのみをサポートし、
  失敗を示す標準的な経路は panic もしくは `SystemMessage::Failure` を明示的に送る方法に限られている。
- remote 実装ではネットワーク接続・ハンドシェイク等の失敗が頻発するため、`?` を利用して自然に
  失敗を伝播できる仕組みがあると実装効率が高まり、エラーパスのテストも書きやすくなる。

## 提案概要
- `Behavior` に Result を返すハンドラインターフェースを追加し、Opt-in で利用できるようにする。
  - 例: `Behavior::try_receive(|ctx, msg| -> Result<BehaviorDirective, ActorFailure> {...})`
  - 例: `Behavior::try_setup(|ctx| -> Result<Behavior<U, R>, ActorFailure>)`
- `ActorCell` の dispatch 処理で Result を判定し、`Err` 時には `FailureInfo` を構築して guardian 経由で
  スーパービジョンを起動する。
- 既存の `Behavior::receive` 等は内部的に `Ok(…)` を返すラッパーとして実装し、既存コードへの破壊的変更を避ける。
- `ActorFailure` のような標準エラー型を定義し、`From<anyhow::Error>` 等の拡張も検討する。

## 詳細タスク
1. `Behavior` API 拡張
   - `Behavior::try_receive`, `Behavior::try_setup`, `Behavior::try_signal` を追加。
   - 既存の `Behavior` コンストラクタは `Ok` ラップで呼び出すように内部実装を調整。
2. `ActorCell` 更新
   - Dispatch 時に `Result` を取り出し、`Err` の場合は `FailureInfo` を生成して `guardian.notify_failure` へ渡す。
   - panic (`catch_unwind`) 路を既存通り維持しつつ、`Err`/panic 双方を `FailureInfo` として扱う。
3. `FailureInfo` 拡張
   - Result 経由で得たエラー型を `FailureInfo` に含めるためのコンストラクタ・フォーマットを追加。
   - `ActorFailure` の標準実装を検討（少なくとも Display/Debug を保持）。
4. ドキュメント・テスト
   - 新 API の使い方を `docs/design/2025-10-13-behavior-result-handler.md` と README 系に記載。
   - 単体テストで `Err` がスーパービジョンへ通知され、再起動／停止指示が働くことを確認。

## リスク
- API 互換性は保つ方針だが、`Behavior` 内部構造の変更に伴う予期せぬ回帰リスク。
- `ActorFailure` の型設計を過剰に汎用化すると複雑化する可能性があるため、最初は最小限に留める。

## 適用効果
- remote 実装を含む失敗が多いアクターで `?` を活用した自然なエラーハンドリングが可能になる。
- panic に頼らない堅牢なエラーパスを実装しやすくなり、テストが書きやすくなる。
- 今後のクラスタ／embedding シナリオでも共通の失敗チャネルを利用可能。

