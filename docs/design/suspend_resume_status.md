# Suspend/Resume 機能の実装状況分析

**作成日**: 2025-10-27
**重要度**: 🔴 **CRITICAL**
**結論**: ⚠️ **定義のみ存在、実装は不完全**

---

## エグゼクティブサマリー

**重大な発見**: `SystemMessage::Suspend` / `SystemMessage::Resume` は**定義されているが、メールボックスレベルでの実装が欠落**しています。

| 項目 | 状況 | 詳細 |
|-----|------|------|
| **型定義** | ✅ 存在 | `SystemMessage::Suspend` / `SystemMessage::Resume` |
| **優先度設定** | ✅ 存在 | `DEFAULT_PRIORITY + 9` |
| **メールボックス実装** | ❌ **欠落** | ユーザーメッセージをブロックする処理なし |
| **ActorCell 実装** | ❌ **欠落** | Suspend/Resume の特殊処理なし |
| **テストケース** | ⚠️ 誤解を招く | テストは存在するが、実際の suspend 挙動を検証していない |

**結論**: 現在の実装では、**Suspend/Resume は単なる「通常のシステムメッセージ」として actor handler に渡されるだけ**で、**メールボックスがユーザーメッセージの処理を停止する機能は実装されていません**。

---

## 1. 現在の実装状況

### 1.1 SystemMessage の定義

**ファイル**: `modules/actor-core/src/api/mailbox/messages/system_message.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemMessage {
    Watch(ActorId),
    Unwatch(ActorId),
    Stop,
    Failure(FailureInfo),
    Restart,
    Suspend,  // ← 定義は存在
    Resume,   // ← 定義は存在
    Escalate(FailureInfo),
    ReceiveTimeout,
}

impl SystemMessage {
    pub fn priority(&self) -> i8 {
        match self {
            | SystemMessage::Suspend | SystemMessage::Resume => DEFAULT_PRIORITY + 9,
            // ...
        }
    }
}
```

**状況**: ✅ 型定義と優先度は存在

### 1.2 ActorCell での処理

**ファイル**: `modules/actor-core/src/internal/actor/actor_cell.rs:241-292`

```rust
pub(super) fn dispatch_envelope(
    &mut self,
    envelope: PriorityEnvelope<AnyMessage>,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
        return Ok(());
    }

    // Stop の特殊処理
    let should_stop = matches!(
        envelope.system_message(),
        Some(SystemMessage::Stop)
    ) && Self::should_mark_stop_for_message();

    // Escalate の特殊処理
    if let Some(SystemMessage::Escalate(failure)) = envelope.system_message().cloned() {
        if let Some(next_failure) = guardian.escalate_failure(failure)? {
            escalations.push(next_failure);
        }
        return Ok(());
    }

    // ❌ Suspend/Resume の特殊処理は存在しない
    // ↓ すべてのメッセージ（Suspend/Resume 含む）が handler に渡される
    let (message, priority) = envelope.into_parts();
    let handler_result = (self.handler)(&mut ctx, message);
    // ...
}
```

**問題点**:
- ❌ `SystemMessage::Suspend` を受け取っても、**ユーザーメッセージの処理を停止しない**
- ❌ `SystemMessage::Resume` を受け取っても、**ユーザーメッセージの処理を再開しない**
- ⚠️ Suspend/Resume は単なる「通常のメッセージ」として handler に渡されるだけ

### 1.3 テストケースの誤解

**ファイル**: `modules/actor-core/src/api/actor/tests.rs:717-757`

```rust
let system_handler = move |_ctx: &mut ActorContext<'_, '_, u32, _>, sys_msg: SystemMessage| {
    if matches!(sys_msg, SystemMessage::Suspend) {
        *failures_clone.borrow_mut() += 1;
    }
};

// ...

// Send system message (Suspend doesn't stop the actor)
actor_ref.send_system(SystemMessage::Suspend).expect("send suspend");
block_on(root.dispatch_next()).expect("dispatch system");

// Verify stateful behavior updated correctly
assert_eq!(*count.borrow(), 15, "State should accumulate user messages");
assert_eq!(*failures.borrow(), 1, "State should track system messages");
```

**問題点**:
- ⚠️ テストコメントに「Suspend doesn't stop the actor」とあるが、これは**誤解を招く**
- ⚠️ 実際には「Suspend は **actor handler に渡されるが、メールボックスの処理を停止しない**」が正しい
- ❌ テストは Suspend メッセージが handler に届くことしか検証していない
- ❌ **Suspend 後にユーザーメッセージが処理されないこと**を検証していない

---

## 2. 旧実装（nexus-actor-rs）の Suspend/Resume

### 2.1 MailboxSuspensionState

**ファイル**: `docs/sources/nexus-actor-rs/modules/actor-std/src/actor/dispatch/mailbox/default_mailbox.rs:209-258`

```rust
#[derive(Debug, Default)]
struct MailboxSuspensionState {
    flag: AtomicBool,                    // suspend 状態フラグ
    since: Mutex<Option<Instant>>,       // suspend 開始時刻
    total_nanos: AtomicU64,              // 累積 suspend 時間
    resume_events: AtomicU64,            // resume 回数
}

impl MailboxSuspensionState {
    fn set(&self, suspended: bool) {
        let was = self.flag.swap(suspended, Ordering::SeqCst);
        if suspended {
            if !was {
                let mut guard = self.since.lock();
                *guard = Some(Instant::now());
            }
        } else if was {
            let mut guard = self.since.lock();
            if let Some(started) = guard.take() {
                let duration = started.elapsed();
                self.record_resume(duration);
            }
        }
    }

    fn is_suspended(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}
```

### 2.2 メールボックス処理での Suspend チェック

**ファイル**: nexus-actor-rs の `run()` メソッド（推定）

```rust
fn run(&self) {
    loop {
        // System メッセージを優先処理
        if let Some(msg) = self.system_mailbox.pop() {
            match msg {
                SystemMessage::Suspend => {
                    self.suspension.set(true);  // ← suspend 状態に設定
                },
                SystemMessage::Resume => {
                    self.suspension.set(false); // ← resume 状態に設定
                },
                _ => self.invoker.invoke_system_message(msg),
            }
            continue;
        }

        // ❗ suspend 中はユーザーメッセージをスキップ
        if self.suspension.is_suspended() {
            return;  // ← ユーザーメッセージを処理しない
        }

        // ユーザーメッセージ処理
        if let Some(msg) = self.user_mailbox.pop() {
            self.invoker.invoke_user_message(msg);
        } else {
            return;  // キューが空なら終了
        }
    }
}
```

**重要な動作**:
1. ✅ `SystemMessage::Suspend` を受信 → `suspension.set(true)`
2. ✅ suspend 状態では**ユーザーメッセージの処理をスキップ**
3. ✅ `SystemMessage::Resume` を受信 → `suspension.set(false)`
4. ✅ resume 後にユーザーメッセージの処理を再開

---

## 3. protoactor-go の Suspend/Resume

**ファイル**: `docs/sources/protoactor-go/actor/mailbox.go:127-177`

```go
func (m *defaultMailbox) run() {
    for {
        // System メッセージを優先処理
        if msg = m.systemMailbox.Pop(); msg != nil {
            atomic.AddInt32(&m.sysMessages, -1)
            switch msg.(type) {
            case *SuspendMailbox:
                atomic.StoreInt32(&m.suspended, 1)  // ← suspend
            case *ResumeMailbox:
                atomic.StoreInt32(&m.suspended, 0)  // ← resume
            default:
                m.invoker.InvokeSystemMessage(msg)
            }
            continue
        }

        // ❗ suspend 中はユーザーメッセージをスキップ
        if atomic.LoadInt32(&m.suspended) == 1 {
            return  // ← ユーザーメッセージを処理しない
        }

        // ユーザーメッセージ処理
        if msg = m.userMailbox.Pop(); msg != nil {
            atomic.AddInt32(&m.userMessages, -1)
            m.invoker.InvokeUserMessage(msg)
        } else {
            return
        }
    }
}
```

**重要な動作**:
- ✅ suspend 状態を atomic flag で管理
- ✅ suspend 中は**ユーザーメッセージの処理をスキップ**し、return でループを抜ける
- ✅ システムメッセージは suspend 中でも処理される

---

## 4. 期待される動作 vs 現在の動作

### 4.1 期待される動作（Akka/Pekko, protoactor-go）

| シーケンス | 期待される動作 |
|-----------|---------------|
| 1. ユーザーメッセージ送信 | ✅ mailbox に enqueue |
| 2. `SystemMessage::Suspend` 送信 | ✅ mailbox に enqueue（優先度高） |
| 3. メールボックス処理 | ✅ Suspend を先に処理 |
| 4. Suspend 処理 | ✅ **suspend フラグを立てる** |
| 5. 後続のユーザーメッセージ | ❌ **処理をスキップ**（mailbox に残る） |
| 6. `SystemMessage::Resume` 送信 | ✅ mailbox に enqueue（優先度高） |
| 7. Resume 処理 | ✅ **suspend フラグを降ろす** |
| 8. 後続のユーザーメッセージ | ✅ **処理を再開** |

### 4.2 現在の動作（cellex-rs）

| シーケンス | 現在の動作 |
|-----------|-----------|
| 1. ユーザーメッセージ送信 | ✅ mailbox に enqueue |
| 2. `SystemMessage::Suspend` 送信 | ✅ mailbox に enqueue（優先度高） |
| 3. メールボックス処理 | ✅ Suspend を先に処理 |
| 4. Suspend 処理 | ⚠️ **actor handler に渡されるだけ** |
| 5. 後続のユーザーメッセージ | ❌ **通常通り処理される**（suspend されない！） |
| 6. `SystemMessage::Resume` 送信 | ✅ mailbox に enqueue（優先度高） |
| 7. Resume 処理 | ⚠️ **actor handler に渡されるだけ** |
| 8. 後続のユーザーメッセージ | ✅ 通常通り処理される |

**問題**: Suspend/Resume が**メールボックスレベルで機能していない**

---

## 5. 影響範囲

### 5.1 機能的影響

| 機能 | 影響 | 詳細 |
|-----|------|-----|
| **バックプレッシャ制御** | 🔴 不可能 | メールボックスを一時停止できない |
| **Stashing との連携** | 🔴 不可能 | suspend 中にメッセージを保留できない |
| **レート制限** | 🔴 不可能 | アクターを一時的に停止できない |
| **初期化待機** | 🔴 不可能 | 初期化完了まで処理を保留できない |
| **動的負荷調整** | 🔴 不可能 | 過負荷時にアクターを一時停止できない |

### 5.2 Akka/Pekko との互換性

| 機能 | Akka/Pekko | cellex-rs | ギャップ |
|-----|-----------|-----------|---------|
| Suspend/Resume | ✅ 完全実装 | ❌ 未実装 | **大きなギャップ** |
| Stashing | ✅ 完全実装 | ❌ 未実装 | **大きなギャップ** |
| バックプレッシャ | ✅ Suspend 経由 | ❌ 未実装 | **大きなギャップ** |

**結論**: 現在の cellex-rs は、**Akka/Pekko の重要な制御機能が欠落**しています。

---

## 6. リファクタリングプランでの扱い

### 6.1 `actor_scheduler_refactor.md` での言及

**セクション 4.4**: `InvokeResult` に `Suspended` バリアントが定義されている

```rust
pub enum InvokeResult {
    Completed { ready_hint: bool },
    Yielded,
    Suspended {
        reason: SuspendReason,
        resume_on: ResumeCondition,
    },  // ← Suspended を返すことで suspend を表現
    Failed { error: String, retry_after: Option<Duration> },
    Stopped,
}
```

**セクション 7**: Suspend/Resume の責務配置について言及

> Suspend 状態の mail box 着信や異常時のガーディアン連携など主要な分岐を明示し、エッジケースをアーキテクチャレベルで把握できるようにする。

**セクション 7 - オープン課題 P0**:

> **P0**: Suspend/Resume の責務配置を Invoker 内に固定するかの判断 (Phase 0)
> ReadyQueueCoordinator が状態を持たない方針を ADR で確定

**分析**:
- ✅ リファクタリングプランで Suspend/Resume は**認識されている**
- ⚠️ しかし、**現在の実装が不完全であることは明記されていない**
- ⚠️ Phase 0 の P0 課題として挙げられているが、「既存実装の完成」ではなく「新しい設計の決定」として扱われている

---

## 7. 推奨される実装

### 7.1 最小限の実装（Phase 0〜1）

**ステップ 1**: ActorCell に suspend 状態を追加

```rust
pub struct ActorCell<MF, Strat> {
    // ... 既存フィールド
    suspended: AtomicBool,  // ← 追加
    suspend_since: Mutex<Option<Instant>>,  // ← 統計用
}
```

**ステップ 2**: `dispatch_envelope` で Suspend/Resume を特殊処理

```rust
pub(super) fn dispatch_envelope(
    &mut self,
    envelope: PriorityEnvelope<AnyMessage>,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
        return Ok(());
    }

    // ❗ Suspend/Resume の特殊処理を追加
    match envelope.system_message() {
        Some(SystemMessage::Suspend) => {
            self.suspended.store(true, Ordering::SeqCst);
            let mut guard = self.suspend_since.lock();
            *guard = Some(Instant::now());
            return Ok(());  // ← handler には渡さない
        }
        Some(SystemMessage::Resume) => {
            self.suspended.store(false, Ordering::SeqCst);
            let mut guard = self.suspend_since.lock();
            if let Some(since) = guard.take() {
                let duration = since.elapsed();
                // メトリクス記録
            }
            return Ok(());  // ← handler には渡さない
        }
        _ => {}
    }

    // ❗ suspend 中はユーザーメッセージをスキップ
    if self.suspended.load(Ordering::SeqCst) && envelope.system_message().is_none() {
        // ユーザーメッセージを mailbox に戻す、または処理をスキップ
        return Ok(());
    }

    // 通常処理
    // ...
}
```

**ステップ 3**: `process_pending` で suspend チェック

```rust
pub(crate) fn process_pending(
    &mut self,
    guardian: &mut Guardian<MF, Strat>,
    new_children: &mut Vec<ActorCell<MF, Strat>>,
    escalations: &mut Vec<FailureInfo>,
) -> Result<usize, QueueError<PriorityEnvelope<AnyMessage>>> {
    if self.stopped {
        return Ok(0);
    }

    // ❗ suspend 中はシステムメッセージのみ処理
    if self.suspended.load(Ordering::SeqCst) {
        // システムメッセージのみを処理するロジック
        return Ok(self.process_system_messages_only(guardian, escalations)?);
    }

    // 通常処理
    let envelopes = self.collect_envelopes()?;
    // ...
}
```

### 7.2 完全な実装（Phase 2B）

**リファクタリングプラン統合**:

`InvokeResult::Suspended` を活用した実装：

```rust
impl MessageInvoker for ActorCellInvoker {
    fn invoke_batch(&mut self, max_messages: usize) -> InvokeResult {
        // suspend 状態を先に評価
        if self.actor_cell.is_suspended() {
            return InvokeResult::Suspended {
                reason: SuspendReason::UserDefined,
                resume_on: ResumeCondition::ExternalSignal(self.resume_signal_key),
            };
        }

        // 通常のメッセージ処理
        // ...
    }
}
```

**ReadyQueueCoordinator での処理**:

```rust
impl ReadyQueueCoordinator for DefaultCoordinator {
    fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult) {
        match result {
            InvokeResult::Suspended { reason, resume_on } => {
                // ready queue から除外
                self.unregister(idx);
                // resume 条件を登録
                self.register_resume_condition(idx, resume_on);
            }
            InvokeResult::Completed { ready_hint: true } => {
                self.register_ready(idx);  // 再登録
            }
            // ...
        }
    }
}
```

---

## 8. 推奨アクション

### 8.1 即座に実施（優先度: 🔴 最高）

1. **ドキュメント訂正**
   - `mailbox_akka_pekko_comparison.md` の Suspend/Resume 評価を「❌ 未実装」に修正
   - `actor_scheduler_refactor_claude_review.md` の Suspend/Resume 評価を訂正

2. **Issue 作成**
   - タイトル: 「Suspend/Resume 機能が未実装」
   - 優先度: P0（最高）
   - 説明: 本レポートの内容を要約

3. **テストケース修正**
   - `modules/actor-core/src/api/actor/tests.rs:717-757` のテストを修正
   - Suspend 後にユーザーメッセージが**処理されないこと**を検証

### 8.2 Phase 0 で実施（優先度: 🔴 高）

4. **最小限の Suspend/Resume 実装**
   - ActorCell に `suspended: AtomicBool` を追加
   - `dispatch_envelope` で Suspend/Resume を特殊処理
   - `process_pending` で suspend チェック
   - 単体テスト 10 ケース追加

5. **ADR 作成**
   - `docs/adr/2025-10-27-suspend-resume-implementation.md`
   - 設計判断と実装方針を文書化

### 8.3 Phase 2B で実施（優先度: 🟡 中）

6. **完全な Suspend/Resume 実装**
   - `InvokeResult::Suspended` との統合
   - ReadyQueueCoordinator での suspend 状態管理
   - Resume 条件（ExternalSignal / After / WhenCapacityAvailable）の実装

---

## 9. 結論

### 9.1 現状評価

| 項目 | 評価 | 理由 |
|-----|------|-----|
| **型定義** | ⭐⭐⭐⭐⭐ (5.0) | SystemMessage に存在 |
| **実装完全性** | ⭐☆☆☆☆ (1.0) | **メールボックスレベルでの実装が欠落** |
| **テストカバレッジ** | ⭐☆☆☆☆ (1.0) | 誤解を招くテストのみ |
| **Akka/Pekko 互換性** | ⭐☆☆☆☆ (1.0) | **重要な制御機能が欠落** |

### 9.2 最終回答

> **質問**: 旧実装では suspend, resume を対応していました。この手の機能は不要なの？現在の実装にはないよね？
>
> **回答**:
> 1. ❌ **不要ではありません。非常に重要な機能です。**
> 2. ✅ **ご指摘の通り、現在の実装には実質的に存在しません。**
>
> **詳細**:
> - `SystemMessage::Suspend` / `SystemMessage::Resume` は**定義されている**
> - しかし、**メールボックスレベルでユーザーメッセージの処理を停止する実装が欠落**
> - 現在は単なる「通常のメッセージ」として actor handler に渡されるだけ
> - **旧実装（nexus-actor-rs）には完全な実装があった**
>
> **影響**:
> - バックプレッシャ制御が不可能
> - Stashing との連携が不可能
> - レート制限・初期化待機が不可能
> - **Akka/Pekko との大きな互換性ギャップ**
>
> **推奨**: Phase 0 で最小限の実装を完成させ、Phase 2B で完全な実装に拡張すべき。

---

**レポート作成者**: Claude (Sonnet 4.5)
**作成日**: 2025-10-27
**重要度**: 🔴 **CRITICAL** - 即座の対応が必要
