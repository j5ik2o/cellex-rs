# コレクション統一リファクタリング計画

## 目的
- `modules/utils-core` を厳格に `no_std`（`alloc` のみ）で完結させ、環境依存コードを `utils-std` / `utils-embedded` に分離する。
- Queue / Stack 系の API・モジュール構造・命名を揃え、Storage → Backend → Shared（ArcShared） → Queue/Stack API の層構造を明示化する。
- トレイト責務を整理し、利用者が多様なバックエンドを組み合わせやすい抽象を提供する。

## 実装方針（TDD 原則）

* **小さく動かしながら進める**
  1 機能 1 テスト（red → green → refactor）の単位で作業し、常にテストがグリーンの状態を保つ。
  破壊的変更や抽象追加は、既存テストをすべて通した状態でマージする。

* **テスト先行で仕様を固定化する**
  新しい抽象（Storage／Backend／Shared／Facade）やポリシー（`OverflowPolicy`, `StackOverflowPolicy` 等）は、まず失敗するテストを作り、その期待値を仕様として確定させる。
  仕様が曖昧な箇所はテスト記述時点でレビューし、**テストが通る＝仕様が固まった**という状態を作る。

* **常時 CI 検証を回す**
  `cargo check --no-default-features --features alloc`、`thumbv6m/7em` の cross check、`cargo test -p collections-next-core` を必須ゲートにする。
  新規テストを追加するたびに CI を通し、**赤のままコミットしない**。

> 目的は「常に動く main」を保ちながら仕様を具体化していくこと。
> 各フェーズの成果はテストで保証され、計画書と仕様書のズレを最小化する。

## 既存実装の参照方針

- 旧 `queue` / `stack` 実装（`modules/utils-core/src/collections/queue` / `modules/utils-core/src/collections/stack`）の意味論を事前に把握し、振る舞いと制約を理解したうえで再設計に反映する。
- 特別な理由が無い限り、挙動が同等であることを優先し既存コード片を流用する（流用時も目的と制約を明文化する）。
- 意味論の変更が必要な場合は、差分の意図と検証方法をテストに落とし込み、レビュアーが判断できる材料を用意する。
- 退避済みの旧世代コード（例: `docs/sources/nexus-actor-rs/`）は参照のみとし、直接流用しない。

## レイヤ責務と抽象

| 層 | 代表トレイト / 型 | 責務 | 具体例（計画時点） |
| --- | --- | --- | --- |
| Storage | `QueueStorage<T>` / `StackStorage<T>` | 生データバッファの読み書き管理（`alloc` のみ、`unsafe` はここに閉じ込める） | リングバッファ、固定長配列 |
| Backend | `QueueBackend<T>` / `StackBackend<T>` | Storage を操作し offer/poll 等のロジックを提供（常に `&mut self`、同期は担当しない） | リングインデックス管理、ヒープベース優先度制御 |
| Shared | `ArcShared<T>`（薄い共有ラッパ） | Backend を共有しつつ同期を吸収する最小限の型。内部で `cfg` により Arc / Rc / critical-section 等を選択 | `ArcShared<MpscRingBackend<T>>`, `ArcShared<BinaryHeapBackend<T>>` |
| Queue / Stack API | `Queue<T, K, Backend>` / `Stack<T, Backend>` | 利用者向け API。型レベルの区別子（TypeKey）と Backend を組み合わせて offer/poll 等を委譲し、溢れ政策・エラー整合性を保つ | `Queue<T, MpscKey, MpscRingBackend<T>>`, `Queue<T, PriorityKey, BinaryHeapBackend<T>>` |

### 同期戦略
- Backend はどの TypeKey でも同期を持たず、`&mut self` 操作のみを提供する。
- 共有と同期は `ArcShared<T>`（薄いラッパ）に集約する。公開 API は `new`, `with_mut`, `try_with_mut` に限定し、内部実装を `cfg` で以下に切り替える。
- 失敗（Mutex 毒、借用衝突等）は `SharedError` として捕捉し、Queue レイヤで `QueueError` にマップする。Poison → `Disconnected`、BorrowConflict → `WouldBlock`、InterruptContext → `WouldBlock` 等の対応表を ADR に明記する。
- `ArcShared<T>` の `Send` / `Sync` 実装条件（例: 内部が Mutex ベースかつ `T: Send`）と割り込み文脈での使用制約（critical-section 実装は割り込み内再入不可等）を明文化する。
- 公開 API としての `QueueHandle` は廃止し、必要であれば内部（sealed）境界としてのみ保持する。基本方針は queue が `ArcShared<Backend>` を保持する形に統一する。

### TypeKey と契約
- TypeKey は型レベルタグ（`struct MpscKey; impl TypeKey for MpscKey {}` 等）として表現し、コンパイル時に不正な組み合わせを防ぐ。
- 能力トレイト（Capabilities）を導入し、TypeKey ごとに実装する：`trait MultiProducer: TypeKey {}`, `trait SingleProducer: TypeKey {}`, `trait SingleConsumer: TypeKey {}`, `trait SupportsPeek: TypeKey {}` など。
- Backend 側は `where K: MultiProducer` 等の制約を通じて、誤用を型レベルで防ぐ。
- 想定契約と能力対応（Phase1で確定）:
  - `MpscKey`: `MultiProducer + SingleConsumer`。
  - `SpscKey`: `SingleProducer + SingleConsumer`（`Rc<RefCell<_>>` 等の非 `Send` を許容）。
  - `FifoKey`: `SingleProducer + SingleConsumer` を基本とし、固定長リングで FIFO を提供。
  - `PriorityKey`: `SingleProducer + SingleConsumer + SupportsPeek`（初期実装）。`PriorityBackend<T>` を通じて `peek_min` 等を公開し、将来的に `MultiProducer` 対応を検討。
- Phase1 の成果物で TypeKey と能力トレイトの対応表、API 差分をまとめる。

### 容量・溢れ政策・エラー
- `OverflowPolicy`（例: `DropNewest` / `DropOldest` / `Block` / `Grow`）を導入し、構築時に選択（既定値は `DropOldest`）。`offer` の戻り値は `Result<OfferOutcome, QueueError>` とし、成功時に `OfferOutcome::Enqueued` / `OfferOutcome::DroppedOldest { count }` / `OfferOutcome::DroppedNewest { count }` / `OfferOutcome::GrewTo { capacity }` などの情報を返す。
- `QueueError` は `Full`, `Empty`, `Closed`, `Disconnected`, `WouldBlock`, `AllocError` を最低限提供し、TypeKey/Backend 間で意味を揃える。
- `ArcShared` 由来の失敗は `SharedError` → `QueueError` にマップする。この際、Poison → `Disconnected`、BorrowConflict → `WouldBlock`、InterruptContext → `WouldBlock` 等、対応表を ADR に明記する。
**既定値（Default）は `OverflowPolicy::DropOldest`。**

### KPI とベンチマーク
- KPI: offer/poll の p50/p99 レイテンシ、スループット（Mpsc: producer 数 1/2/4/8、Spsc: 1/1）、Priority のヒープサイズ別性能。
- Phase1 でベースラインを取得し、以降 ±5% 以内を維持することを目標にする（誤差許容の根拠も ADR に記録）。
- ベンチマークは `criterion` を中心に、必要に応じて `iai-callgrind` 等で命令数も確認する。
  - フェーズごとにシナリオと測定条件（容量、ペイロードサイズ、CPU、ターゲット）を固定。


## 実装モジュール構成（新設計）

- 既存 `queue` / `stack` は現状の API を維持し、互換性を壊さない。
- 新設計は `modules/utils-core/src/collections/queue2` と `stack2` に実装する。
- 旧 API との移行期間中は両方並立させ、`queue2`/`stack2` の API を安定化後に統合可否を再評価する。

## フェーズ構成（SP はストーリーポイント）

**各フェーズは着手前にテスト設計レビューを行い、主要仕様をテストで先に固定する。**

### フェーズ1: 現状調査と設計整理（見積り: 3pt）
- Queue / Stack 配下の型・トレイト・依存関係を図式化し、レイヤ責務表・TypeKey 契約案・OverflowPolicy/エラー一覧を作成。
- `no_std` 制約下で維持すべき抽象セットと、統廃合する既存トレイト（`QueueRw` 等）の扱いを決定。
- 現行機能を利用しているクレート（actor-core 等）への影響を棚卸し、影響ファイルリストを作成。
- ベースライン計測: `ring_queue_offer_poll` など主要操作の latency/throughput を記録し KPI 化。
- 成果物: ADR（仮 `ADR-queue-refactor`）に同期戦略（ArcShared 切替条件）、TypeKey 契約、OverflowPolicy、KPI、エラー方針を明記。

### フェーズ2: Core 抽象の再編（見積り: 8pt）
1. トレイト整備 (3pt): `QueueBackend` / `QueueStorage` を再設計し、Stack 側も同構成へ揃える。既存トレイト（`QueueRw` 等）の統廃合方針を決定し、共有層は `ArcShared<T>` へ一本化する。
2. Queue API 再編 (3pt): `Queue<T, K, Backend>` を導入し、既存の `RingQueue` 等を薄い型へ置換。`TypeKey` を導入し、型レベルで契約を表現。
3. Priority 対応 (2pt): Priority 用 Backend/Storage 抽象を切り出し、`PriorityBackend<T: Ord>` のような専用トレイトを定義。Queue 側は `where B: PriorityBackend<T>` を通じて `peek_min` 等の操作を公開する。
4. core 側ユニットテストは `alloc` のみで動作するダミー Backend を用意し、溢れ政策・エラー遷移・TypeKey 契約を検証。`cargo +stable test -p cellex-utils-core-rs` / `makers ci-check` を通しながら段階的に移行。

### フェーズ3a: 環境別 Backend 実装（見積り: 7pt）
1. `modules/utils-std` に std 依存 Backend（`StdRingBackend`、`StdPriorityBackend` 等）を新設し、`modules/utils-embedded` には組み込み向け Backend（`CriticalSectionBackend` など）を配置。core からは同期原語や `Arc` 依存コードを排除。
2. `ArcShared` の `cfg` 切替実装を整備し、Send/Sync 条件・割り込み規約をテスト（std 側は loom、embedded 側は critical-section で実確認）。
3. モジュール構成の整備: 直属親のみ再エクスポートする形へ調整し、module wiring lint をパス。

### フェーズ3b: 互換 API と移行支援（見積り: 6pt）
1. 移行支援: 旧 API には `#[deprecated(note = "... use SharedQueue<MpscKey> ...")]` を付与し、暫定エイリアスとマイグレーションガイド（コード例付き）を提供。利用箇所をフェーズごとに差し替え。
2. 主要クレート（actor-core など）で新 API へ移行し、ビルドを確認。
3. ベンチ更新: Phase1 で取得したベースラインと比較し、性能回帰をチェック。

### フェーズ4: 仕上げとドキュメント（見積り: 5pt）
- 旧 API を整理し `QueueBackend` 系抽象に一本化。`cargo check --no-default-features --features alloc` と thumb ターゲット向け `cargo check` で `no_std` 回帰を検証。
- `makers ci-check` と `cargo make coverage` を完走させ、性能回帰がないか確認。
- Queue 設計に関するドキュメント／ADR／ミグレーションガイドを更新し、新構造とエラーハンドリング方針（例: `Result<(), QueueError<E>>` を基本とする）を明示。Priority/FIFO でのエラー差異もまとめる。

## 補足方針
- 破壊的変更は小さめの PR/ブランチに分割し、段階的にレビューを進める。
- CI には `cargo check --no-default-features --features alloc` を追加し、`no_std` 制約の逸脱を自動検知。
- 命名は責務が分かるものへ改訂し（例: `MpscQueue` → `SharedQueue` + `MpscBackend`）、利用者が抽象レイヤを意識できるようにする。
- KPI として offer/poll レイテンシ・スループットを Phase ごとに比較し、±5% 以内を維持することを目標にする。
- エラーハンドリングは `QueueError` を中心とした `Result` ベースに統一し、`no_std` で扱いやすい設計を前提とする。
- `ArcShared` の実装は shared モジュールに閉じ込め、Send/Sync 実装条件と `cfg` の切替条件を ADR に記録する。
- 競合検証（`loom` 等）は `utils-std` 側で実施し、core では `alloc` のみで完結するプロパティテスト・ベンチを重視する。
- TypeKey の命名は `*Key` に統一（例: `MpscKey`, `SpscKey`, `FifoKey`, `PriorityKey`）し、将来的な追加も同スキームに従う。
- v2 実装は現行コードへ影響を与えないよう `modules/utils-core/src/v2/` 以下に新規配置し、既存 `src/collections` の構造や公開 API を段階移行完了までは変更しない。
### ランタイム拡張の想定
- コア（queue2 / stack2）は `no_std + alloc` 前提で実装し、同期は `ArcShared` の抽象に委譲する。
- ランタイム固有の Backend / Shared 実装（Tokio, Embassy 等）は `utils-std`・`utils-embedded` など環境別クレートで提供する余地を残す。
- Tokio 向けに `TokioMpscBackend`、Embassy 向けに `CriticalSectionBackend` 等を別実装として追加できるよう、core では `QueueBackend` / `StackBackend` の trait 境界を狭めない。
- TypeKey / Capability により API 表面が安定するため、ランタイム別 Backend を差し替えても利用者コードは最小の変更で済む。
