#!/usr/bin/env python3
"""
Benchmark Comparison Script for ActorScheduler Refactoring

ベースラインと現在のベンチマーク結果を比較し、パフォーマンス回帰を検出します。

使用方法:
    python3 scripts/compare_benchmarks.py \\
        --baseline benchmarks/baseline_before_refactor.md \\
        --current target/criterion/

出力:
    - 標準出力: テキストフォーマットの比較結果
    - benchmark_comparison.md: マークダウン形式のレポート
    - 終了コード: 0=正常, 1=回帰検出, 2=エラー
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple
import re


class BenchmarkResult:
    """ベンチマーク結果を表すクラス"""

    def __init__(self, name: str, mean: float, std_dev: float, unit: str = "ns"):
        self.name = name
        self.mean = mean
        self.std_dev = std_dev
        self.unit = unit

    def __repr__(self):
        return f"{self.name}: {self.mean:.2f} ±{self.std_dev:.2f} {self.unit}"


def parse_criterion_json(criterion_dir: Path) -> Dict[str, BenchmarkResult]:
    """
    Criterion 出力ディレクトリから JSON ファイルを解析

    Args:
        criterion_dir: target/criterion/ ディレクトリのパス

    Returns:
        ベンチマーク名をキーとした結果の辞書
    """
    results = {}

    # すべての benchmark.json ファイルを探索
    for json_file in criterion_dir.rglob("new/estimates.json"):
        try:
            with open(json_file, "r") as f:
                data = json.load(f)

            # ベンチマーク名を抽出（ディレクトリ構造から）
            # 例: target/criterion/mailbox_throughput/bounded_1000/new/estimates.json
            #     -> mailbox_throughput/bounded_1000
            # criterion/ の次から new/ の前までを結合
            parts = json_file.parts
            try:
                criterion_idx = parts.index("criterion")
                new_idx = parts.index("new")
                benchmark_name = "/".join(parts[criterion_idx + 1:new_idx])
            except (ValueError, IndexError):
                print(f"Warning: Could not parse benchmark name from {json_file}", file=sys.stderr)
                continue

            mean = data.get("mean", {}).get("point_estimate", 0)
            std_dev = data.get("std_dev", {}).get("point_estimate", 0)

            results[benchmark_name] = BenchmarkResult(
                name=benchmark_name, mean=mean, std_dev=std_dev, unit="ns"
            )

        except (json.JSONDecodeError, KeyError) as e:
            print(f"Warning: Failed to parse {json_file}: {e}", file=sys.stderr)
            continue

    return results


def parse_baseline_markdown(baseline_file: Path) -> Dict[str, BenchmarkResult]:
    """
    ベースラインマークダウンファイルからベンチマーク結果を解析

    マークダウンフォーマット例:
    | Benchmark | Mean | Std Dev |
    |-----------|------|---------|
    | mailbox_throughput/bounded_1000 | 1234.56 ns | 12.34 ns |

    Args:
        baseline_file: ベースラインマークダウンファイルのパス

    Returns:
        ベンチマーク名をキーとした結果の辞書
    """
    results = {}

    with open(baseline_file, "r") as f:
        content = f.read()

    # テーブル行を抽出（|で始まり|で終わる行）
    table_pattern = r'\|\s*(.+?)\s*\|\s*([\d.]+)\s*ns\s*\|\s*([\d.]+)\s*ns\s*\|'

    for match in re.finditer(table_pattern, content):
        name = match.group(1).strip()
        if name == "Benchmark" or name.startswith("-"):
            continue  # ヘッダー行とセパレータ行をスキップ

        mean = float(match.group(2))
        std_dev = float(match.group(3))

        results[name] = BenchmarkResult(name=name, mean=mean, std_dev=std_dev)

    return results


def compare_results(
    baseline: Dict[str, BenchmarkResult], current: Dict[str, BenchmarkResult]
) -> List[Tuple[str, float, bool]]:
    """
    ベースラインと現在の結果を比較

    Args:
        baseline: ベースライン結果
        current: 現在の結果

    Returns:
        (ベンチマーク名, 変化率(%), 回帰フラグ) のリスト
    """
    comparisons = []

    for name in sorted(set(baseline.keys()) | set(current.keys())):
        if name not in baseline:
            # 新規ベンチマーク
            comparisons.append((name, None, False))
            continue

        if name not in current:
            # 削除されたベンチマーク
            comparisons.append((name, None, True))
            continue

        baseline_mean = baseline[name].mean
        current_mean = current[name].mean

        # 変化率を計算（正: 悪化, 負: 改善）
        if baseline_mean > 0:
            change_percent = ((current_mean - baseline_mean) / baseline_mean) * 100
        else:
            change_percent = 0

        # 5% 以上の悪化を回帰と判定
        is_regression = change_percent > 5.0

        comparisons.append((name, change_percent, is_regression))

    return comparisons


def generate_report(
    comparisons: List[Tuple[str, float, bool]], output_file: Path
) -> None:
    """
    比較結果をマークダウンレポートとして生成

    Args:
        comparisons: compare_results() の出力
        output_file: 出力ファイルパス
    """
    with open(output_file, "w") as f:
        f.write("# Benchmark Comparison Report\n\n")
        f.write("## Summary\n\n")

        total_count = len(comparisons)
        regression_count = sum(1 for _, _, is_reg in comparisons if is_reg)
        improvement_count = sum(
            1 for _, change, _ in comparisons if change is not None and change < -5.0
        )

        f.write(f"- **Total Benchmarks**: {total_count}\n")
        f.write(f"- **Regressions (>5% slower)**: {regression_count}\n")
        f.write(f"- **Improvements (>5% faster)**: {improvement_count}\n\n")

        f.write("## Detailed Results\n\n")
        f.write("| Benchmark | Change | Status |\n")
        f.write("|-----------|--------|--------|\n")

        for name, change, is_regression in sorted(
            comparisons, key=lambda x: x[1] if x[1] is not None else 0, reverse=True
        ):
            if change is None:
                status = "⚠️ New or Removed"
                change_str = "N/A"
            elif is_regression:
                status = "🔴 Regression"
                change_str = f"+{change:.2f}%"
            elif change < -5.0:
                status = "🟢 Improvement"
                change_str = f"{change:.2f}%"
            else:
                status = "⚪ No Change"
                change_str = f"{change:+.2f}%"

            f.write(f"| {name} | {change_str} | {status} |\n")

        f.write("\n---\n")
        f.write("Generated by scripts/compare_benchmarks.py\n")


def print_summary(comparisons: List[Tuple[str, float, bool]]) -> None:
    """
    比較結果のサマリを標準出力に表示

    Args:
        comparisons: compare_results() の出力
    """
    print("\n" + "=" * 70)
    print("BENCHMARK COMPARISON SUMMARY")
    print("=" * 70 + "\n")

    regressions = [
        (name, change) for name, change, is_reg in comparisons
        if is_reg and change is not None
    ]

    if regressions:
        print("⚠️  REGRESSIONS DETECTED (>5% slower):\n")
        for name, change in sorted(regressions, key=lambda x: x[1], reverse=True):
            print(f"  🔴 {name}: +{change:.2f}%")
        print()
    else:
        print("✅ No significant regressions detected.\n")

    improvements = [
        (name, change)
        for name, change, _ in comparisons
        if change is not None and change < -5.0
    ]

    if improvements:
        print("IMPROVEMENTS (>5% faster):\n")
        for name, change in sorted(improvements, key=lambda x: x[1]):
            print(f"  🟢 {name}: {change:.2f}%")
        print()

    print("=" * 70 + "\n")


def main():
    parser = argparse.ArgumentParser(
        description="Compare benchmark results and detect regressions"
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        required=True,
        help="Path to baseline markdown file (e.g., benchmarks/baseline_before_refactor.md)",
    )
    parser.add_argument(
        "--current",
        type=Path,
        required=True,
        help="Path to current criterion output directory (e.g., target/criterion/)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("benchmark_comparison.md"),
        help="Output report file (default: benchmark_comparison.md)",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=5.0,
        help="Regression threshold percentage (default: 5.0)",
    )

    args = parser.parse_args()

    # ベースライン解析
    if not args.baseline.exists():
        print(f"Error: Baseline file not found: {args.baseline}", file=sys.stderr)
        return 2

    print(f"Parsing baseline: {args.baseline}")
    baseline_results = parse_baseline_markdown(args.baseline)
    print(f"  Found {len(baseline_results)} baseline benchmarks")

    # 現在の結果解析
    if not args.current.exists():
        print(f"Error: Current results directory not found: {args.current}", file=sys.stderr)
        return 2

    print(f"Parsing current results: {args.current}")
    current_results = parse_criterion_json(args.current)
    print(f"  Found {len(current_results)} current benchmarks")

    # 比較
    comparisons = compare_results(baseline_results, current_results)

    # レポート生成
    generate_report(comparisons, args.output)
    print(f"\nReport generated: {args.output}")

    # サマリ表示
    print_summary(comparisons)

    # 回帰検出時は終了コード1で終了
    has_regression = any(is_reg for _, _, is_reg in comparisons)
    return 1 if has_regression else 0


if __name__ == "__main__":
    sys.exit(main())
