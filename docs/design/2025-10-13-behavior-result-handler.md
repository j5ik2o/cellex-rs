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
- `ActorFailure` は `Box<dyn BehaviorFailure>` を保持する薄いコンテナとし、エラー情報は `BehaviorFailure`
  トレイトで抽象化する。`From<E>` 実装などで任意のエラー型を `BehaviorFailure` 実装経由で包む。

## データモデル拡張
- `BehaviorFailure` トレイトを新設し、Supervisor 戦略が型ダウンキャストできるよう `as_any` を必須メソッドとする。
- 標準実装として文字列メッセージを包む `DefaultBehaviorFailure` を用意し、
  `BehaviorFailure` を実装していないエラー型も簡単に包めるようにする。
  - `DefaultBehaviorFailure` は `message: Cow<'static, str>` と `debug: Option<String>` を保持し、
    Supervisor で共通に参照できる最小限の情報セットを提供する。
- `ActorFailure` は Display/Debug を委譲しつつ `Box<dyn BehaviorFailure + Send + Sync>` を保持する構造体として再設計する。
- これによりカスタム Supervisor 戦略は `failure.behavior().as_any().downcast_ref::<FooError>()` のように具象型へアクセス可能。
- 既存の文字列のみのエラー表現は廃止し、少なくとも原因・オリジナルのエラー型・付随情報をトレイト経由で取得できるようにする。
- Supervisor 戦略の `fn decide(&mut self, actor: ActorId, error: &dyn fmt::Debug) -> SupervisorDirective` シグネチャは
  `&dyn BehaviorFailure` を受け取る形に変更し、常に挙動一致を保証する。後方互換性は不要とし、新シグネチャに一本化する。
  - 旧来の `fmt::Debug` ベースのシグネチャは廃止し、`BehaviorFailure` の存在を境界として明示することで
    Supervisor レイヤが常にダウンキャスト可能な値を扱えるようにする。
  - `decide` へ渡す値は `FailureInfo::behavior_failure()` から取得する統一経路を用意し、`panic` や `Result::Err`
    の双方で同じ変換を経るようにする。

```rust
use std::borrow::Cow;

pub trait BehaviorFailure: std::fmt::Debug + Send + Sync + 'static {
  fn as_any(&self) -> &dyn std::any::Any;

  fn description(&self) -> Cow<'_, str> {
    Cow::Owned(format!("{:?}", self))
  }
}

pub struct ActorFailure {
  inner: Box<dyn BehaviorFailure>,
}

impl ActorFailure {
  pub fn new(inner: impl BehaviorFailure) -> Self {
    Self { inner: Box::new(inner) }
  }

  pub fn behavior(&self) -> &dyn BehaviorFailure {
    self.inner.as_ref()
  }
}
```

## FailureInfo と変換経路
- `Result::Err(ActorFailure)` で得られた値はそのまま `FailureInfo::from_behavior_failure` で保持し、Supervisor へ渡す。
- panic 捕捉 (`catch_unwind`) では `Box<dyn Any + Send>` をパターンマッチし、
  - 既に `BehaviorFailure` を実装した型であればそのままラップ
  - `&'static str`/`String`/`Cow<'static, str>` はメッセージとして `DefaultBehaviorFailure` で包む
  - その他の型（例: 任意の `T: Any + Send`）は `Debug` 表現を採用した `PanicBehaviorFailure`（`DefaultBehaviorFailure` 派生）に変換
  する専用ヘルパーを提供する。
- `FailureInfo` 構造体は `BehaviorFailure` の参照を必ず引き渡せるように持ち方を再設計する（例: `Arc<ActorFailure>` を保持）。
- 既存の文字列ベース `FailureInfo` フォーマットは削除し、`BehaviorFailure` を前提とした API を揃える。

## 詳細タスク
1. `Behavior` API 拡張
   - `Behavior::try_receive`, `Behavior::try_setup`, `Behavior::try_signal` を追加。
   - 既存の `Behavior` コンストラクタは `Ok` ラップで呼び出すように内部実装を調整。
2. `BehaviorFailure` インターフェース整備
   - `BehaviorFailure` トレイトを定義し、`fn as_any(&self) -> &dyn Any` を必須とする。
   - 表示用の `fmt_message` などはデフォルト実装を提供し、最低限 `Debug`/`Display` での出力が保証されるようにする。
   - `DefaultBehaviorFailure`（仮称）で文字列と `Debug` 表現を包み、`From<E>` 実装で自動ラップできるようにする。
3. `ActorCell` 更新
   - Dispatch 時に `Result` を取り出し、`Err` の場合は `FailureInfo` を生成して `guardian.notify_failure` へ渡す。
   - panic (`catch_unwind`) 路を既存通り維持しつつ、`Err`/panic 双方を `FailureInfo` として扱う。
4. `SupervisorStrategy` 更新
   - `decide(&mut self, actor: ActorId, error: &dyn BehaviorFailure) -> SupervisorDirective` にシグネチャ変更。
   - 既存の `fmt::Debug` 参照を受け取っていた呼び出し箇所は全て `BehaviorFailure` を渡すように調整し、後方互換性は考慮しない。
5. `FailureInfo` 拡張
   - `FailureInfo` が `ActorFailure` 内の `BehaviorFailure` トレイトオブジェクトへ直接アクセスできるよう API を追加。
   - panic 由来のペイロードを `BehaviorFailure` に変換するヘルパー（例: `FailureInfo::from_panic_payload`）を実装。
   - Supervisor から具象ダウンキャストするヘルパー（例: `fn downcast_ref<T>(&self) -> Option<&T>`）を提供。
6. ルーティング整備
   - Guardian や Mailbox など `FailureInfo` を経由するすべての経路で `ActorFailure` を必ず保持・伝播するよう確認。
   - 既存の `Debug` ベースの通知ロジックは削除し、新トレイト準拠に揃える。
7. ドキュメント・テスト
   - 新 API の使い方を `docs/design/2025-10-13-behavior-result-handler.md` と README 系に記載。
   - 単体テストで `Err` がスーパービジョンへ通知され、再起動／停止指示が働くことを確認。

## リスク
- API 互換性は保つ方針だが、`Behavior` 内部構造の変更に伴う予期せぬ回帰リスク。
- `BehaviorFailure` トレイトを導入することで型安全性が増す一方、トレイトオブジェクトのライフタイム・送受信制約
  （`Send + Sync + 'static`）を満たす必要があり実装側の負担が増える。
- `ActorFailure` の型設計を過剰に汎用化すると複雑化する可能性があるため、デフォルト実装を整備しつつ過剰な API 展開は避ける。
- `panic` 起因の `FailureInfo` では `Box<dyn Any + Send>` から `BehaviorFailure` へ変換するラッパーが必須となり、
  `panic` の `Message` と `ActorFailure` の両立をどう扱うか追加検討が必要。最低限 `panic` payload を文字列化した
  `DefaultBehaviorFailure` によるラップを提供し、Supervisor 戦略からは統一的に `BehaviorFailure` として扱えるようにする。
- `FailureInfo` を `BehaviorFailure` 前提へ移行することで、guardian から remote 連携まで含めた影響範囲が広がる。
  すべての呼び出しが新 API へ移行したかを網羅的にテストする必要がある。

## 適用効果
- remote 実装を含む失敗が多いアクターで `?` を活用した自然なエラーハンドリングが可能になる。
- panic に頼らない堅牢なエラーパスを実装しやすくなり、テストが書きやすくなる。
- 今後のクラスタ／embedding シナリオでも共通の失敗チャネルを利用可能。
