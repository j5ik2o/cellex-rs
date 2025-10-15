# ActorSystem エントリポイント簡素化計画 (2025-10-15)

## 背景
- 現行の `ActorSystem` 初期化は `ActorRuntimeRunner`・`TokioSystemHandle` など複数コンポーネントの組み合わせが必要で、学習コストが高い。
- `root_context.spawn(props)` を任意に呼べるため、トップレベルのアクター構成がコードごとに散逸し、Akka Typed のような「Guardian Actor から開始する」設計になっていない。
- ユーザーからは「Akka/Pekko の `ActorSystem(...)` のように最小コードで起動したい」「エントリポイントの責務を明確にしたい」との要望がある。

## 目的
- ランタイム起動時に登場する概念数を減らし、初学者でもコピー＆ペーストで動かせる標準パターンを提供する。
- トップレベルの `Props` を必ず指定させ、Guardian Actor の振る舞いを一貫させる。
- 既存の柔軟な制御（Tokio の LocalSet、独自ランタイム等）は維持しつつ、高レベル API で隠蔽する。

## 既存課題と制約
- `actor-core` は `no_std` を想定するため、Tokio 等の依存を直接持てない。
- 既存ユーザーは `ActorSystemRunner` / `TokioSystemHandle` を利用しているため、互換性に配慮した段階的移行が必要。
- `root_context.spawn` を完全に禁止すると既存テストや内部コードに影響が出るため、非公開化は段階実施が望ましい。

## 提案概要
1. **高レベル起動ファサードの導入**  
   - `actor-std` に `TokioActorSystem::new(props, name)`（非同期 or 同期）を追加し、Tokio runtime 上での起動・終了待ちを一括提供する。  
   - 生成された `TokioActorSystem` からは `tell` / `when_terminated` / `shutdown` を直接呼べるようにし、Akka/Pekko に近い UX を実現する。内部では `TokioSystemHandle` を保持し、従来 API を隠蔽する。

2. **トップレベル Props の強制**  
   - 高レベルファサードで必ずメインアクターの `Props` を受け取り、内部で Guardian として spawn する。  
   - `root_context.spawn` は段階的に非公開化または `#[deprecated]` 指定し、トップレベルはファサード経由に誘導する。

3. **高度な制御は従来 API を併存**  
   - 既存の `GenericActorRuntime`、`ActorSystemRunner`、`TokioSystemHandle` は `pub` のまま維持し、ドキュメントで「高度な制御が必要な場合はこちら」と誘導する。  
   - `GenericActorRuntime` 自体にランタイムハンドルを格納させるのではなく、責務を分離する。

## 詳細設計
### 1. 起動ファサード (`actor-std`)
- 追加 API（案）  
  ```rust
  pub struct TokioActorSystem<M>
  where
    M: Element + 'static,
  {
    handle: TokioSystemHandle<M>,
    main_actor: ActorRef<M, GenericActorRuntime<TokioMailboxRuntime>>,
  }

  impl<M> TokioActorSystem<M>
  where
    M: Element + 'static,
  {
    pub async fn new<P>(props: P, name: &str) -> Result<Self, StartError>
    where
      P: Into<Props<M, GenericActorRuntime<TokioMailboxRuntime>>>,
    {
      // GenericActorRuntime + ActorSystem::builder で Guardian を起動し、
      // ActorSystemRunner + TokioSystemHandle を内部で構築する
    }

    pub fn tell(&self, message: M) -> Result<(), QueueError<M>> {
      self.main_actor.tell(message)
    }

    pub async fn when_terminated(&self) -> Result<(), ShutdownError> {
      self.handle.when_terminated().await
    }

    pub fn shutdown(&self) {
      self.handle.shutdown()
    }
  }
  ```
- 内部処理
  - `GenericActorRuntime::new(TokioMailboxRuntime)` + `ActorSystem::builder` を利用し、`launch_tokio` 的なヘルパーで `TokioSystemHandle` を生成。
  - Guardian Props の `spawn` はファサード内部で完結させ、その `ActorRef` を保持して `tell` へ委譲する。

### 2. `root_context.spawn` の取り扱い
- フェーズ1: `pub` のまま `#[deprecated(note = "Use TokioActorSystem::new or Guardian actor props.")]` を付与し、コンパイル時に警告を出す。
- フェーズ2: `pub(crate)` 化し、`actor-core` 内部テストのみ利用。外部向けにはファサード経由か Guardian からの spawn を要求する。
- フェーズ3: 不要な内部利用を削減し、最終的には `RootContext` 自体を外公開しない方向を検討。

### 3. ドキュメント／サンプル整備
- README / guides に「最小コード例」として `TokioActorSystem::new` を掲載。
- 既存サンプルを `root_context.spawn` からファサード利用へ書き換え、利用者に推奨パターンを示す。
- 学習ステップを段階化（基本 → 応用 → 拡張）し、どの API を使えばよいかを明示する。

## 移行プラン
1. `actor-std` に起動ファサードを追加し、ドキュメントとサンプルを更新。
2. `root_context.spawn` へ deprecation を付与し、警告文で新 API を紹介。
3. 外部公開しているサンプル／テストを新 API へ順次移行。
4. 利用者からのフィードバックを踏まえ、`pub(crate)` 化のタイミングを判断。

## リスク・懸念
- 既存コードが `root_context.spawn` へ直接依存している場合の移行コスト。
- Tokio 以外（Embassy 等）のファサード整備を同時に行わないと利用者が混乱する恐れ。
- 高レベル API がランタイム管理を隠蔽することで、上級ユーザーが挙動を追いづらくなる可能性（ドキュメントで補う）。

## 次のアクション
1. `actor-std` に `TokioActorSystem::new`（仮称）を実装し、戻り値のハンドル仕様を確定する。
2. `root_context.spawn` の deprecation を実施し、既存サンプルの差し替え着手。
3. `actor-embedded` 側でも同様のプリセット API（例: `EmbassyActorSystem::start`）の要否を検討する。
