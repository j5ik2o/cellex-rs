# MessageInvoker トレイト設計ドラフト

## 概要

MessageInvokerは、ActorSchedulerリファクタリングにおいてメッセージ実行ロジックを担当するコンポーネントです。このドキュメントでは、Phase 2Bで実装予定のMessageInvokerトレイトの設計素案と参考実装スケッチを提供します。

**ステータス**: Phase 0 ドラフト（Phase 2Bで実装予定）
**作成日**: 2025-10-22
**関連ドキュメント**: `actor_scheduler_refactor.md` Section 4.3-4.4

---

## 1. 責務と設計原則

### 責務

MessageInvokerは以下の責務を持ちます：

1. **メッセージ実行ロジック**: MailboxからEnvelopeをdequeueし、ActorCellに渡して処理する
2. **ミドルウェアチェイン管理**: before_invoke/after_invokeの実行順序を制御する
3. **Suspend/Resume判定**: ActorStateを評価し、適切なInvokeResultを返す
4. **公平性保証**: throughput_hintに基づいて処理を自発的に中断（Yield）する
5. **エラーハンドリング**: ActorErrorを捕捉し、Guardian連携のためのInvokeResult::Failedを返す

### 設計原則

- **単一責任**: メッセージ実行ロジックのみに専念（queueの管理はCoordinatorが担当）
- **ステートレス**: 実行状態はActorCellが保持、Invoker自身は状態を持たない（ミドルウェアスタックを除く）
- **拡張性**: MiddlewareChainトレイトによりカスタムミドルウェアを追加可能
- **テスタビリティ**: MailboxとActorCellへの依存をトレイト経由で注入し、モック可能にする

---

## 2. トレイト定義

### MessageInvoker トレイト

```rust
/// MessageInvoker - メッセージ実行ロジックを抽象化するトレイト
///
/// WorkerExecutorから呼び出され、指定されたMailboxIndexに対して
/// メッセージ処理を実行し、結果をInvokeResultとして返します。
///
/// # Thread Safety
///
/// 実装は `Send` である必要があります（`Sync` は不要）。
/// 各WorkerスレッドがInvokerのインスタンスを独立して所有します。
pub trait MessageInvoker: Send {
  /// メッセージをバッチ処理する
  ///
  /// # Arguments
  ///
  /// * `throughput_hint` - 一度に処理する最大メッセージ数（公平性のため）
  ///
  /// # Returns
  ///
  /// - `InvokeResult::Completed { ready_hint }` - 処理完了
  ///   - `ready_hint = true`: 未処理メッセージが残っている（再登録が必要）
  ///   - `ready_hint = false`: キューが空（再登録不要）
  /// - `InvokeResult::Yielded` - throughput_hintに到達（公平性のため中断）
  /// - `InvokeResult::Suspended { reason, resume_on }` - アクターがサスペンド状態
  /// - `InvokeResult::Failed { error, retry_after }` - 処理中にエラー発生
  /// - `InvokeResult::Stopped` - アクターが停止済み
  fn invoke_batch(&mut self, throughput_hint: usize) -> InvokeResult;

  /// Invokerが対象とするMailboxIndexを取得
  ///
  /// デバッグやメトリクス収集に使用されます。
  fn mailbox_index(&self) -> MailboxIndex;

  /// ミドルウェアを追加する（Phase 2B）
  ///
  /// ミドルウェアは登録順に before_invoke が実行され、
  /// 逆順に after_invoke が実行されます。
  fn add_middleware(&mut self, middleware: Box<dyn MiddlewareChain>);
}
```

### InvokeContext（ミドルウェア用）

```rust
/// InvokeContext - ミドルウェアに渡されるコンテキスト情報
///
/// before_invoke/after_invokeでミドルウェア間の情報共有に使用されます。
pub struct InvokeContext {
  /// 対象のMailboxIndex
  pub idx: MailboxIndex,

  /// throughput hint（一度に処理する最大メッセージ数）
  pub throughput_hint: usize,

  /// 実行開始時刻（after_invokeでのレイテンシ測定用）
  pub start_time: Instant,

  /// ミドルウェア間で共有するメタデータ
  ///
  /// 例: log_id, trace_id, rate_limit_next などを格納
  pub metadata: HashMap<String, Value>,
}
```

### MiddlewareChain トレイト（再掲）

```rust
/// MiddlewareChain - MessageInvoker実装に前後処理を提供するミドルウェアチェイン
///
/// before_invokeは順方向（外→内）、after_invokeは逆方向（内→外）で実行されます。
pub trait MiddlewareChain: Send {
  /// メッセージ処理前に実行
  ///
  /// # Returns
  ///
  /// - `ControlFlow::Continue(())` - 次のミドルウェアまたは本処理へ進む
  /// - `ControlFlow::Break(())` - 処理を中断（Invokerは即座にSuspendedを返す）
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()>;

  /// メッセージ処理後に実行
  ///
  /// before_invokeでBreakした場合でもafter_invokeは実行されます（リソース解放のため）。
  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult);
}
```

---

## 3. 参考実装スケッチ

### ActorCellInvoker（Phase 2B実装予定）

```rust
/// ActorCellInvoker - MessageInvokerのデフォルト実装
///
/// ActorCellとQueueMailboxを保持し、メッセージ処理ループを駆動します。
pub struct ActorCellInvoker {
  /// 対象のMailboxIndex
  idx: MailboxIndex,

  /// QueueMailboxへの参照（キャッシュ）
  mailbox: Arc<QueueMailbox>,

  /// ActorCellへの参照（キャッシュ）
  actor_cell: Arc<ActorCell>,

  /// ミドルウェアスタック
  middleware: Option<CompositeMiddleware>,

  /// Observability Hub（メトリクス送出）
  observability: Arc<ObservabilityHub>,

  /// Guardian通知チャネル（エラー時の連携）
  guardian_tx: mpsc::UnboundedSender<FailureNotification>,
}

impl ActorCellInvoker {
  /// 新しいActorCellInvokerを作成
  ///
  /// # Arguments
  ///
  /// * `idx` - MailboxIndex
  /// * `mailbox` - QueueMailboxへの参照
  /// * `actor_cell` - ActorCellへの参照
  /// * `observability` - Observability Hub
  /// * `guardian_tx` - Guardian通知チャネル
  pub fn new(
    idx: MailboxIndex,
    mailbox: Arc<QueueMailbox>,
    actor_cell: Arc<ActorCell>,
    observability: Arc<ObservabilityHub>,
    guardian_tx: mpsc::UnboundedSender<FailureNotification>,
  ) -> Self {
    Self {
      idx,
      mailbox,
      actor_cell,
      middleware: None,
      observability,
      guardian_tx,
    }
  }

  /// ActorCellの状態を評価
  fn check_actor_state(&self) -> ActorState {
    self.actor_cell.state()
  }

  /// メッセージバッチをdequeue
  fn dequeue_batch(&self, max: usize) -> Vec<Envelope> {
    self.mailbox.dequeue_batch(max)
  }

  /// メッセージバッチを処理
  ///
  /// # Returns
  ///
  /// - `Ok(processed_count)` - 処理成功
  /// - `Err(ActorError)` - 処理中にエラー発生
  fn process_messages_batch(
    &mut self,
    envelopes: Vec<Envelope>,
  ) -> Result<usize, ActorError> {
    let mut processed = 0;

    for envelope in envelopes {
      // ActorCellにEnvelopeを渡して処理
      self.actor_cell.handle_envelope(envelope)?;
      processed += 1;
    }

    Ok(processed)
  }

  /// Guardian へエラーを通知
  fn notify_guardian(&self, error: ActorError) {
    let notification = FailureNotification {
      idx: self.idx,
      error: error.clone(),
      timestamp: Instant::now(),
    };

    // 非ブロッキングで送信（Guardian側で処理）
    let _ = self.guardian_tx.send(notification);
  }

  /// バックオフ時間を算出（連続失敗回数から）
  fn calculate_backoff(&self, failure_count: usize) -> Option<Duration> {
    // 指数バックオフ: 2^n * 100ms (最大10秒)
    let base_ms = 100;
    let backoff_ms = (base_ms * 2_u64.pow(failure_count as u32)).min(10_000);
    Some(Duration::from_millis(backoff_ms))
  }
}

impl MessageInvoker for ActorCellInvoker {
  fn invoke_batch(&mut self, throughput_hint: usize) -> InvokeResult {
    let start_time = Instant::now();

    // Step 1: InvokeContext作成
    let mut ctx = InvokeContext {
      idx: self.idx,
      throughput_hint,
      start_time,
      metadata: HashMap::new(),
    };

    // Step 2: before_invoke ミドルウェアチェイン（順方向）
    if let Some(ref mut middleware) = self.middleware {
      match middleware.before_invoke(&ctx) {
        ControlFlow::Continue(_) => {
          // 処理を続行
        }
        ControlFlow::Break(_) => {
          // ミドルウェアが処理を保留
          // （例: RateLimitMiddlewareがトークン不足を検出）

          // after_invoke は Break でも実行（リソース解放）
          let result = InvokeResult::Suspended {
            reason: SuspendReason::RateLimit,
            resume_on: ResumeCondition::After(
              ctx
                .metadata
                .get("rate_limit_next")
                .and_then(|v| v.as_duration())
                .unwrap_or(Duration::from_millis(100)),
            ),
          };
          middleware.after_invoke(&ctx, &result);

          return result;
        }
      }
    }

    // Step 3: ActorState評価
    let actor_state = self.check_actor_state();

    match actor_state {
      ActorState::Suspended => {
        // アクターがサスペンド状態 → Suspended を返す
        let result = InvokeResult::Suspended {
          reason: self.actor_cell.suspend_reason(),
          resume_on: self.actor_cell.resume_condition(),
        };

        // メトリクス送出
        self
          .observability
          .metric(Metric::SuspendCount { idx: self.idx });

        // after_invoke ミドルウェアチェイン
        if let Some(ref mut middleware) = self.middleware {
          middleware.after_invoke(&ctx, &result);
        }

        return result;
      }
      ActorState::Stopped => {
        // アクターが停止済み → Stopped を返す
        let result = InvokeResult::Stopped;

        if let Some(ref mut middleware) = self.middleware {
          middleware.after_invoke(&ctx, &result);
        }

        return result;
      }
      ActorState::Stopping | ActorState::Running => {
        // 処理を続行
      }
    }

    // Step 4: メッセージバッチをdequeue
    let envelopes = self.dequeue_batch(throughput_hint);
    let envelope_count = envelopes.len();

    if envelope_count == 0 {
      // キューが空 → Completed { ready_hint: false }
      let result = InvokeResult::Completed { ready_hint: false };

      // メトリクス送出
      self.observability.metric(Metric::InvokeDuration {
        idx:      self.idx,
        duration: start_time.elapsed(),
      });

      if let Some(ref mut middleware) = self.middleware {
        middleware.after_invoke(&ctx, &result);
      }

      return result;
    }

    // メトリクス: dequeue成功
    self.observability.metric(Metric::DequeueCount {
      idx:   self.idx,
      count: envelope_count,
    });

    // Step 5: メッセージバッチを処理
    let result = match self.process_messages_batch(envelopes) {
      Ok(processed_count) => {
        // 処理成功

        // 公平性チェック: throughput_hint に達したか？
        if processed_count >= throughput_hint {
          // 未処理メッセージがある可能性が高い → Yielded
          InvokeResult::Yielded
        } else {
          // throughput_hint未満で処理完了
          // → キューにまだメッセージがあるか確認
          let has_more = self.mailbox.has_messages();

          InvokeResult::Completed {
            ready_hint: has_more,
          }
        }
      }
      Err(actor_error) => {
        // 処理中にエラー発生

        // Guardianに通知（非同期）
        self.notify_guardian(actor_error.clone());

        // 連続失敗回数を取得（ActorCellが保持）
        let failure_count = self.actor_cell.consecutive_failure_count();

        // バックオフ時間を算出
        let retry_after = self.calculate_backoff(failure_count);

        // メトリクス: エラーカウント
        self.observability.metric(Metric::FailureCount {
          idx:   self.idx,
          error: actor_error.to_string(),
        });

        InvokeResult::Failed {
          error: actor_error.to_string(),
          retry_after,
        }
      }
    };

    // Step 6: メトリクス送出
    self.observability.metric(Metric::InvokeDuration {
      idx:      self.idx,
      duration: start_time.elapsed(),
    });

    // Step 7: after_invoke ミドルウェアチェイン（逆方向）
    if let Some(ref mut middleware) = self.middleware {
      middleware.after_invoke(&ctx, &result);
    }

    result
  }

  fn mailbox_index(&self) -> MailboxIndex {
    self.idx
  }

  fn add_middleware(&mut self, middleware: Box<dyn MiddlewareChain>) {
    if let Some(ref mut composite) = self.middleware {
      composite.add(middleware);
    } else {
      self.middleware = Some(CompositeMiddleware::new(vec![middleware]));
    }
  }
}
```

### CompositeMiddleware（ミドルウェア合成）

```rust
/// CompositeMiddleware - 複数のミドルウェアを合成
///
/// before_invokeは順方向、after_invokeは逆方向で実行されます。
pub struct CompositeMiddleware {
  middlewares: Vec<Box<dyn MiddlewareChain>>,
}

impl CompositeMiddleware {
  pub fn new(middlewares: Vec<Box<dyn MiddlewareChain>>) -> Self {
    Self { middlewares }
  }

  pub fn add(&mut self, middleware: Box<dyn MiddlewareChain>) {
    self.middlewares.push(middleware);
  }
}

impl MiddlewareChain for CompositeMiddleware {
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()> {
    // 順方向実行（外 → 内）
    for middleware in &mut self.middlewares {
      match middleware.before_invoke(ctx) {
        ControlFlow::Continue(_) => continue,
        ControlFlow::Break(_) => return ControlFlow::Break(()),
      }
    }
    ControlFlow::Continue(())
  }

  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult) {
    // 逆方向実行（内 → 外）
    for middleware in self.middlewares.iter_mut().rev() {
      middleware.after_invoke(ctx, result);
    }
  }
}
```

---

## 4. 標準ミドルウェア例（Phase 2B実装予定）

### TelemetryMiddleware

```rust
/// TelemetryMiddleware - OpenTelemetry span/metricsの記録
pub struct TelemetryMiddleware {
  tracer: Arc<Tracer>,
}

impl MiddlewareChain for TelemetryMiddleware {
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()> {
    // Span開始
    let span = self.tracer.start("invoke_batch");
    ctx.metadata.insert("span", Value::Span(span));

    ControlFlow::Continue(())
  }

  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult) {
    // Span終了
    if let Some(Value::Span(span)) = ctx.metadata.get("span") {
      span.end();
    }

    // メトリクス記録
    let duration = ctx.start_time.elapsed();
    metrics::histogram!("invoke.duration_ms", duration.as_millis() as f64);

    if matches!(result, InvokeResult::Failed { .. }) {
      metrics::counter!("invoke.error_count", 1);
    }
  }
}
```

### LoggingMiddleware

```rust
/// LoggingMiddleware - 構造化ログの記録
pub struct LoggingMiddleware;

impl MiddlewareChain for LoggingMiddleware {
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()> {
    let log_id = uuid::Uuid::new_v4();
    ctx.metadata.insert("log_id", Value::Uuid(log_id));

    tracing::debug!(
      idx = ?ctx.idx,
      log_id = %log_id,
      "Invoking mailbox"
    );

    ControlFlow::Continue(())
  }

  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult) {
    let log_id = ctx.metadata.get("log_id").and_then(|v| v.as_uuid());

    tracing::debug!(
      idx = ?ctx.idx,
      log_id = ?log_id,
      result = ?result,
      "Completed mailbox"
    );
  }
}
```

### RateLimitMiddleware

```rust
/// RateLimitMiddleware - トークンバケットによるレート制限
pub struct RateLimitMiddleware {
  token_bucket: Arc<Mutex<TokenBucket>>,
}

impl MiddlewareChain for RateLimitMiddleware {
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()> {
    let mut bucket = self.token_bucket.lock().unwrap();

    match bucket.try_acquire(1) {
      Ok(_) => {
        // トークン取得成功
        ControlFlow::Continue(())
      }
      Err(next_available_at) => {
        // トークン不足 → 処理を保留
        ctx
          .metadata
          .insert("rate_limit_next", Value::Duration(next_available_at));

        metrics::counter!("rate_limit.triggered", 1);

        ControlFlow::Break(())
      }
    }
  }

  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult) {
    // 使用したトークン数をメトリクスに記録
    metrics::counter!("rate_limit.tokens_consumed", 1);
  }
}
```

---

## 5. WorkerExecutorとの連携

WorkerExecutorは以下のようにMessageInvokerを使用します：

```rust
impl WorkerExecutor {
  pub async fn run(&self) {
    loop {
      // 1. シグナル待機
      self.coordinator.poll_wait_signal(&mut cx).await;

      // 2. Ready queueから取り出し
      let mut batch = Vec::new();
      self.coordinator.drain_ready_cycle(self.max_batch, &mut batch);

      // 3. 各インデックスに対してInvokerを作成・実行
      for idx in batch {
        // Invokerを作成（キャッシュから取得 or 新規作成）
        let mut invoker = self.get_or_create_invoker(idx);

        // メッセージ処理実行
        let result = invoker.invoke_batch(self.throughput_hint);

        // 4. 結果をCoordinatorに通知
        self.coordinator.handle_invoke_result(idx, result);
      }
    }
  }

  fn get_or_create_invoker(&self, idx: MailboxIndex) -> ActorCellInvoker {
    // Invokerキャッシュから取得、なければ新規作成
    self
      .invoker_cache
      .get(&idx)
      .cloned()
      .unwrap_or_else(|| {
        // MailboxRegistryから必要な参照を取得
        let mailbox = self.registry.get_mailbox(idx).expect("valid index");
        let actor_cell = self.registry.get_actor_cell(idx).expect("valid index");

        // Invoker作成
        ActorCellInvoker::new(
          idx,
          mailbox,
          actor_cell,
          self.observability.clone(),
          self.guardian_tx.clone(),
        )
      })
  }
}
```

---

## 6. テスト戦略

### 単体テスト（Phase 2B）

```rust
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_invoke_batch_empty_queue() {
    // キューが空の場合、Completed { ready_hint: false } を返すことを確認
  }

  #[test]
  fn test_invoke_batch_suspended_actor() {
    // ActorStateがSuspendedの場合、InvokeResult::Suspendedを返すことを確認
  }

  #[test]
  fn test_invoke_batch_yielded_at_throughput_hint() {
    // throughput_hintに達したらYieldedを返すことを確認
  }

  #[test]
  fn test_invoke_batch_error_handling() {
    // ActorErrorが発生した場合、InvokeResult::Failedを返すことを確認
  }

  #[test]
  fn test_middleware_before_invoke_break() {
    // before_invokeでBreakが返された場合、処理が保留されることを確認
  }

  #[test]
  fn test_middleware_after_invoke_order() {
    // after_invokeが逆順で実行されることを確認
  }

  #[test]
  fn test_guardian_notification_on_error() {
    // エラー時にGuardianへ通知が送られることを確認
  }
}
```

### 統合テスト（Phase 2B）

```rust
#[tokio::test]
async fn test_invoker_with_coordinator_integration() {
  // WorkerExecutor + ReadyQueueCoordinator + MessageInvoker の統合テスト
  // 100アクター × 1000メッセージで正常動作することを確認
}

#[tokio::test]
async fn test_suspend_resume_cycle() {
  // Suspend → Resume のサイクルが正しく動作することを確認
  // （ADR-002のテストケース）
}
```

---

## 7. Phase 2B 実装タスク

MessageInvoker実装のタスク分解：

1. **トレイト定義** (`ready_queue_coordinator.rs`に追記)
   - [ ] `MessageInvoker` トレイト定義
   - [ ] `InvokeContext` struct定義
   - [ ] `MiddlewareChain` トレイト定義（既にPhase 0で定義済み）

2. **ActorCellInvoker実装** (`message_invoker.rs` 新規作成)
   - [ ] 基本構造体定義
   - [ ] `invoke_batch` メソッド実装
   - [ ] ActorState評価ロジック
   - [ ] メッセージバッチ処理ループ
   - [ ] エラーハンドリングとGuardian連携
   - [ ] バックオフ計算ロジック

3. **ミドルウェア実装** (`middleware/` ディレクトリ新規作成)
   - [ ] `CompositeMiddleware` 実装
   - [ ] `TelemetryMiddleware` 実装
   - [ ] `LoggingMiddleware` 実装
   - [ ] `RateLimitMiddleware` 実装

4. **WorkerExecutor統合** (`worker_executor.rs` 修正)
   - [ ] Invokerキャッシュ機構
   - [ ] `get_or_create_invoker` メソッド
   - [ ] invoke_batch呼び出し

5. **テスト** (`message_invoker/tests.rs` 新規作成)
   - [ ] 単体テスト 7ケース
   - [ ] Guardian連携テスト 5ケース
   - [ ] ミドルウェアテスト 7ケース
   - [ ] 統合テスト 5ケース

---

## 8. オープン課題

### P1: ActorCellとの境界

- **課題**: ActorCellの公開APIからメッセージ実行関連メソッドをどこまで削減するか？
- **案**:
  - `handle_envelope` は残す（Invokerから呼び出すため）
  - `process_messages` などの高レベルメソッドは削除候補
  - `actor_state()`, `suspend_reason()`, `resume_condition()` は残す
- **決定**: Phase 2B 開始時にActorCellのAPI棚卸しを実施

### P2: Invokerキャッシュのライフサイクル

- **課題**: WorkerExecutorがInvokerをキャッシュする場合、Actorが停止したときにどう削除するか？
- **案**:
  - MailboxRegistryがunregister時にコールバックを発火
  - WorkerExecutorがコールバックを受けてキャッシュから削除
  - または、InvokeResult::Stoppedを受け取ったときに削除
- **決定**: Phase 2B 実装時に決定

### P3: ミドルウェアの動的追加

- **課題**: 実行時にミドルウェアを追加・削除できるようにするか？
- **案**:
  - Phase 2Bでは静的（Invoker作成時に固定）
  - Phase 3以降で動的追加APIを検討
- **決定**: Phase 2Bでは静的に限定

---

## 9. 参照

- [actor_scheduler_refactor.md](./actor_scheduler_refactor.md) - 全体設計
- [ADR-001: 命名ポリシー](../adr/2025-10-22-phase0-naming-policy.md) - Invoker命名の根拠
- [ADR-002: Suspend/Resume 責務配置](../adr/2025-10-22-phase0-suspend-resume-responsibility.md) - Suspend判定ロジック
- [scheduler_sequences.puml](./scheduler_sequences.puml) - ミドルウェアフロー図
- [scheduler_implementation_faq.md](./scheduler_implementation_faq.md) - Q3-2（ミドルウェア実装）

---

**最終更新**: 2025-10-22
**フェーズ**: Phase 0（ドラフト）
**次回更新**: Phase 2B 開始時
