#!/usr/bin/env bash
# Memory Statistics Collection Script for ActorScheduler Benchmarks
#
# ベンチマーク実行時のメモリ使用統計を収集し、レポートを生成します。
#
# 使用方法:
#   ./ci/scripts/collect_memory_stats.sh [benchmark_name]
#
# 出力:
#   - memory_stats.json: JSON形式のメモリ統計
#   - memory_stats.md: マークダウン形式のレポート
#
# 依存:
#   - /usr/bin/time (GNU time推奨、macOSの場合は brew install gnu-time)
#   - cargo
#   - jq (JSONパース用、オプショナル)

set -euo pipefail

# ===========================
# Configuration
# ===========================

BENCHMARK_NAME="${1:-mailbox_throughput}"
OUTPUT_DIR="${OUTPUT_DIR:-target/memory-stats}"
JSON_OUTPUT="${OUTPUT_DIR}/memory_stats.json"
MD_OUTPUT="${OUTPUT_DIR}/memory_stats.md"
TEMP_DIR=$(mktemp -d)

# GNU timeのパス検出（macOS対応）
if command -v gtime &> /dev/null; then
    TIME_CMD="gtime"
elif command -v /usr/bin/time &> /dev/null; then
    # Linux/GNUの場合は /usr/bin/time を使用
    TIME_CMD="/usr/bin/time"
else
    echo "Error: GNU time not found. Install with: brew install gnu-time (macOS) or apt-get install time (Linux)"
    exit 1
fi

# ===========================
# Functions
# ===========================

# メモリ統計を収集する関数
collect_memory_stats() {
    local bench_name="$1"
    local time_output="${TEMP_DIR}/time_output.txt"

    echo "Collecting memory statistics for benchmark: ${bench_name}"

    # GNU timeでベンチマークを実行し、メモリ統計を取得
    # Format: https://www.gnu.org/software/time/manual/html_node/Format.html
    ${TIME_CMD} -v cargo bench --bench "${bench_name}" --features std -- --save-baseline memory-test 2>&1 | tee "${time_output}" || true

    # time_outputからメモリ統計を抽出
    local max_rss=$(grep "Maximum resident set size" "${time_output}" | awk '{print $NF}')
    local avg_rss=$(grep "Average resident set size" "${time_output}" | awk '{print $NF}')
    local page_faults=$(grep "Major (requiring I/O) page faults" "${time_output}" | awk '{print $NF}')
    local minor_faults=$(grep "Minor (reclaiming a frame) page faults" "${time_output}" | awk '{print $NF}')
    local voluntary_switches=$(grep "Voluntary context switches" "${time_output}" | awk '{print $NF}')
    local involuntary_switches=$(grep "Involuntary context switches" "${time_output}" | awk '{print $NF}')

    # JSON出力を生成
    cat > "${JSON_OUTPUT}" <<EOF
{
  "benchmark_name": "${bench_name}",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "memory": {
    "max_rss_kb": ${max_rss:-0},
    "max_rss_mb": $(echo "scale=2; ${max_rss:-0} / 1024" | bc),
    "avg_rss_kb": ${avg_rss:-0},
    "avg_rss_mb": $(echo "scale=2; ${avg_rss:-0} / 1024" | bc)
  },
  "page_faults": {
    "major": ${page_faults:-0},
    "minor": ${minor_faults:-0}
  },
  "context_switches": {
    "voluntary": ${voluntary_switches:-0},
    "involuntary": ${involuntary_switches:-0}
  }
}
EOF

    echo "Memory statistics saved to: ${JSON_OUTPUT}"
}

# Cargoビルドサイズ統計を収集
collect_build_stats() {
    echo "Collecting build size statistics..."

    # リリースビルドを実行
    cargo build --release --quiet

    # バイナリサイズを取得
    local binary_size=$(du -h target/release/cellex-actor-core-rs 2>/dev/null | awk '{print $1}' || echo "N/A")

    # デバッグシンボル除去後のサイズ
    if command -v strip &> /dev/null; then
        cp target/release/cellex-actor-core-rs "${TEMP_DIR}/stripped_binary" 2>/dev/null || true
        strip "${TEMP_DIR}/stripped_binary" 2>/dev/null || true
        local stripped_size=$(du -h "${TEMP_DIR}/stripped_binary" 2>/dev/null | awk '{print $1}' || echo "N/A")
    else
        local stripped_size="N/A"
    fi

    # JSON出力に追加（jqがあれば使用、なければ手動で追記）
    if command -v jq &> /dev/null; then
        jq ". + {\"build\": {\"binary_size\": \"${binary_size}\", \"stripped_size\": \"${stripped_size}\"}}" "${JSON_OUTPUT}" > "${TEMP_DIR}/updated.json"
        mv "${TEMP_DIR}/updated.json" "${JSON_OUTPUT}"
    else
        echo "  (jq not found, skipping build stats integration)"
    fi
}

# マークダウンレポートを生成
generate_markdown_report() {
    local bench_name="$1"

    # JSONから値を読み取り（jqがない場合は手動パース）
    if command -v jq &> /dev/null; then
        local max_rss_mb=$(jq -r '.memory.max_rss_mb' "${JSON_OUTPUT}")
        local avg_rss_mb=$(jq -r '.memory.avg_rss_mb' "${JSON_OUTPUT}")
        local page_faults=$(jq -r '.page_faults.major' "${JSON_OUTPUT}")
        local minor_faults=$(jq -r '.page_faults.minor' "${JSON_OUTPUT}")
        local vol_switches=$(jq -r '.context_switches.voluntary' "${JSON_OUTPUT}")
        local invol_switches=$(jq -r '.context_switches.involuntary' "${JSON_OUTPUT}")
        local timestamp=$(jq -r '.timestamp' "${JSON_OUTPUT}")
    else
        # jqがない場合は簡易パース
        local max_rss_mb=$(grep -o '"max_rss_mb": [0-9.]*' "${JSON_OUTPUT}" | awk '{print $2}')
        local avg_rss_mb=$(grep -o '"avg_rss_mb": [0-9.]*' "${JSON_OUTPUT}" | awk '{print $2}')
        local page_faults=$(grep -o '"major": [0-9]*' "${JSON_OUTPUT}" | head -1 | awk '{print $2}')
        local minor_faults=$(grep -o '"minor": [0-9]*' "${JSON_OUTPUT}" | awk '{print $2}')
        local vol_switches=$(grep -o '"voluntary": [0-9]*' "${JSON_OUTPUT}" | awk '{print $2}')
        local invol_switches=$(grep -o '"involuntary": [0-9]*' "${JSON_OUTPUT}" | awk '{print $2}')
        local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    fi

    cat > "${MD_OUTPUT}" <<EOF
# Memory Statistics Report

## Benchmark Information

- **Benchmark Name**: ${bench_name}
- **Timestamp**: ${timestamp}
- **Host**: $(uname -n)
- **OS**: $(uname -s) $(uname -r)
- **Architecture**: $(uname -m)

## Memory Usage

| Metric | Value |
|--------|-------|
| **Maximum Resident Set Size (RSS)** | ${max_rss_mb} MB |
| **Average Resident Set Size (RSS)** | ${avg_rss_mb} MB |

## Page Faults

| Type | Count |
|------|-------|
| **Major Page Faults (I/O)** | ${page_faults} |
| **Minor Page Faults (Reclaim)** | ${minor_faults} |

## Context Switches

| Type | Count |
|------|-------|
| **Voluntary Context Switches** | ${vol_switches} |
| **Involuntary Context Switches** | ${invol_switches} |

## Analysis

### Memory Efficiency
$(if [ "${max_rss_mb%.*}" -lt 100 ]; then
    echo "✅ **Good**: Peak memory usage is below 100 MB"
elif [ "${max_rss_mb%.*}" -lt 500 ]; then
    echo "⚠️ **Moderate**: Peak memory usage is ${max_rss_mb} MB"
else
    echo "🔴 **High**: Peak memory usage exceeds 500 MB"
fi)

### Page Fault Analysis
$(if [ "${page_faults}" -lt 10 ]; then
    echo "✅ **Good**: Very few major page faults (${page_faults})"
elif [ "${page_faults}" -lt 100 ]; then
    echo "⚠️ **Moderate**: ${page_faults} major page faults detected"
else
    echo "🔴 **High**: Excessive major page faults (${page_faults}) - possible I/O bottleneck"
fi)

### Concurrency Performance
$(if [ "${invol_switches}" -lt 1000 ]; then
    echo "✅ **Good**: Low involuntary context switches (${invol_switches})"
elif [ "${invol_switches}" -lt 10000 ]; then
    echo "⚠️ **Moderate**: ${invol_switches} involuntary context switches"
else
    echo "🔴 **High**: Excessive involuntary switches (${invol_switches}) - CPU contention"
fi)

---

Generated by \`ci/scripts/collect_memory_stats.sh\`
EOF

    echo "Markdown report saved to: ${MD_OUTPUT}"
}

# ===========================
# Main Execution
# ===========================

main() {
    # 出力ディレクトリを作成
    mkdir -p "${OUTPUT_DIR}"

    echo "========================================"
    echo "Memory Statistics Collection"
    echo "========================================"
    echo "Benchmark: ${BENCHMARK_NAME}"
    echo "Output Directory: ${OUTPUT_DIR}"
    echo ""

    # メモリ統計を収集
    collect_memory_stats "${BENCHMARK_NAME}"

    # ビルド統計を収集
    collect_build_stats

    # マークダウンレポートを生成
    generate_markdown_report "${BENCHMARK_NAME}"

    echo ""
    echo "========================================"
    echo "Collection Complete"
    echo "========================================"
    echo "JSON Output: ${JSON_OUTPUT}"
    echo "Markdown Report: ${MD_OUTPUT}"

    # レポート内容をプレビュー
    if [ -f "${MD_OUTPUT}" ]; then
        echo ""
        echo "--- Report Preview ---"
        head -20 "${MD_OUTPUT}"
        echo "..."
    fi

    # 一時ディレクトリをクリーンアップ
    rm -rf "${TEMP_DIR}"
}

# スクリプトを実行
main "$@"
