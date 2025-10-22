# CI Scripts

ActorScheduler リファクタリング用のCI/CDスクリプト集

## Scripts

### collect_memory_stats.sh

ベンチマーク実行時のメモリ使用統計を収集し、分析レポートを生成します。

#### 機能

- **メモリ使用量の測定**: 最大/平均RSS（Resident Set Size）
- **ページフォルト分析**: Major/Minor page faults
- **コンテキストスイッチ統計**: Voluntary/Involuntary context switches
- **ビルドサイズ統計**: バイナリサイズと最適化後サイズ
- **自動レポート生成**: JSON形式とマークダウン形式

#### 依存関係

**必須**:
- `cargo` - Rustビルドツール
- GNU `time` - メモリ統計収集用
  - macOS: `brew install gnu-time`
  - Linux: 通常プリインストール済み

**オプショナル**:
- `jq` - JSON処理（あればより詳細な統計が可能）
  - macOS: `brew install jq`
  - Linux: `apt-get install jq` / `yum install jq`

#### 使用方法

```bash
# 基本的な使用法（デフォルトベンチマーク: mailbox_throughput）
./ci/scripts/collect_memory_stats.sh

# 特定のベンチマークを指定
./ci/scripts/collect_memory_stats.sh scheduler_latency

# 出力ディレクトリを指定
OUTPUT_DIR=/tmp/memory-stats ./ci/scripts/collect_memory_stats.sh mailbox_throughput
```

#### 出力

**JSON形式** (`target/memory-stats/memory_stats.json`):
```json
{
  "benchmark_name": "mailbox_throughput",
  "timestamp": "2025-10-22T12:34:56Z",
  "memory": {
    "max_rss_kb": 51200,
    "max_rss_mb": 50.00,
    "avg_rss_kb": 40960,
    "avg_rss_mb": 40.00
  },
  "page_faults": {
    "major": 5,
    "minor": 1234
  },
  "context_switches": {
    "voluntary": 567,
    "involuntary": 89
  }
}
```

**マークダウン形式** (`target/memory-stats/memory_stats.md`):
- メモリ使用量サマリ
- ページフォルト統計
- コンテキストスイッチ統計
- 自動分析（Good/Moderate/High判定）

#### CI統合例

**GitHub Actions**:
```yaml
- name: Collect memory statistics
  run: |
    ./ci/scripts/collect_memory_stats.sh mailbox_throughput
    ./ci/scripts/collect_memory_stats.sh scheduler_latency

- name: Upload memory stats
  uses: actions/upload-artifact@v3
  with:
    name: memory-statistics
    path: target/memory-stats/
```

**GitLab CI**:
```yaml
memory_stats:
  script:
    - ./ci/scripts/collect_memory_stats.sh mailbox_throughput
  artifacts:
    paths:
      - target/memory-stats/
    expire_in: 30 days
```

#### トラブルシューティング

##### macOSで "time: command not found" エラー

macOSの組み込み`time`はシェルビルトインであり、GNU timeの`-v`オプションをサポートしていません。

**解決方法**:
```bash
brew install gnu-time
```

スクリプトは自動的に`gtime`を検出して使用します。

##### Linux で "Maximum resident set size" が取得できない

一部のLinuxディストリビューションでは、GNU timeがプリインストールされていない場合があります。

**解決方法**:
```bash
# Debian/Ubuntu
sudo apt-get install time

# RHEL/CentOS
sudo yum install time
```

##### jq がない場合

`jq`がインストールされていない場合、ビルド統計の統合がスキップされますが、基本的なメモリ統計は収集されます。

**推奨**: 完全な機能を使用するには`jq`をインストールしてください。

#### メトリクスの解釈

**Maximum Resident Set Size (RSS)**:
- プロセスが使用した物理メモリの最大値
- **良い**: < 100 MB
- **普通**: 100-500 MB
- **高い**: > 500 MB（最適化を検討）

**Major Page Faults**:
- ディスクI/Oが必要なページフォルト
- **良い**: < 10
- **普通**: 10-100
- **高い**: > 100（I/Oボトルネックの可能性）

**Involuntary Context Switches**:
- OSによって強制的に実行を中断されたスレッド切り替え
- **良い**: < 1,000
- **普通**: 1,000-10,000
- **高い**: > 10,000（CPU競合の可能性）

## 関連ドキュメント

- [ActorScheduler Refactor Design](../../docs/design/actor_scheduler_refactor.md)
- [Benchmark Comparison Script](../../scripts/compare_benchmarks.py)
- [Rollback Procedure](../../docs/migration/scheduler_refactor_rollback.md)
