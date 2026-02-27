#!/usr/bin/env python3
"""
Compare performance results between noq and upstream quinn.
Generates a markdown table with throughput, latency, and CPU metrics.
"""

import json
import os
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Optional


@dataclass
class PerfResult:
    scenario: str
    impl: str
    test_type: str  # 'raw' or 'netsim'
    condition: Optional[str]  # network condition for netsim
    upload_mbps: float
    download_mbps: float
    cpu_avg: float
    cpu_max: float


def parse_quinn_perf_json(path: Path) -> Optional[dict]:
    """Parse quinn-perf JSON output."""
    try:
        with open(path) as f:
            data = json.load(f)

        if not data.get('intervals'):
            return None

        # Calculate average throughput from intervals
        total_send_bps = 0
        total_recv_bps = 0
        count = 0

        for interval in data['intervals']:
            if 'sum' in interval:
                if interval['sum'].get('sender'):
                    total_send_bps += interval['sum'].get('bits_per_second', 0)
                else:
                    total_recv_bps += interval['sum'].get('bits_per_second', 0)
            for stream in interval.get('streams', []):
                if stream.get('sender'):
                    total_send_bps += stream.get('bits_per_second', 0)
                else:
                    total_recv_bps += stream.get('bits_per_second', 0)
            count += 1

        if count == 0:
            return None

        return {
            'upload_mbps': (total_send_bps / count) / 1_000_000,
            'download_mbps': (total_recv_bps / count) / 1_000_000,
        }
    except (json.JSONDecodeError, KeyError, FileNotFoundError):
        return None


def parse_metadata_json(path: Path) -> Optional[dict]:
    """Parse metadata.json for CPU stats."""
    try:
        with open(path) as f:
            return json.load(f)
    except (json.JSONDecodeError, FileNotFoundError):
        return None


def collect_results(raw_dir: Path, netsim_dir: Path) -> list[PerfResult]:
    """Collect all results from raw and netsim directories."""
    results = []

    # Collect raw benchmark results
    if raw_dir.exists():
        for impl_dir in raw_dir.iterdir():
            if not impl_dir.is_dir():
                continue
            impl = impl_dir.name

            for scenario_dir in impl_dir.iterdir():
                if not scenario_dir.is_dir():
                    continue
                scenario = scenario_dir.name

                results_json = scenario_dir / 'results.json'
                metadata_json = scenario_dir / 'metadata.json'

                perf_data = parse_quinn_perf_json(results_json)
                metadata = parse_metadata_json(metadata_json)

                if perf_data:
                    results.append(PerfResult(
                        scenario=scenario,
                        impl=impl,
                        test_type='raw',
                        condition=None,
                        upload_mbps=perf_data['upload_mbps'],
                        download_mbps=perf_data['download_mbps'],
                        cpu_avg=metadata.get('cpu_avg', 0) if metadata else 0,
                        cpu_max=metadata.get('cpu_max', 0) if metadata else 0,
                    ))

    # Collect netsim results
    if netsim_dir.exists():
        for impl_dir in netsim_dir.iterdir():
            if not impl_dir.is_dir():
                continue
            impl = impl_dir.name

            for json_file in impl_dir.glob('*.json'):
                if json_file.name == 'metadata.json':
                    continue

                # Extract condition from filename (e.g., ideal_1_to_1 -> ideal)
                condition = json_file.stem.split('_')[0] if '_' in json_file.stem else json_file.stem

                perf_data = parse_quinn_perf_json(json_file)
                if perf_data:
                    results.append(PerfResult(
                        scenario=f'netsim-{condition}',
                        impl=impl,
                        test_type='netsim',
                        condition=condition,
                        upload_mbps=perf_data['upload_mbps'],
                        download_mbps=perf_data['download_mbps'],
                        cpu_avg=0,
                        cpu_max=0,
                    ))

    return results


def format_delta(iroh_val: float, upstream_val: float) -> str:
    """Format the delta between iroh and upstream values."""
    if upstream_val == 0:
        return "N/A"

    delta_pct = ((iroh_val - upstream_val) / upstream_val) * 100

    if abs(delta_pct) < 1:
        return "~0%"
    elif delta_pct > 0:
        return f"+{delta_pct:.1f}%"
    else:
        return f"{delta_pct:.1f}%"


def generate_markdown(results: list[PerfResult]) -> str:
    """Generate markdown comparison table."""
    lines = []

    # Group by scenario
    iroh_results = {r.scenario: r for r in results if 'noq' in r.impl.lower()}
    upstream_results = {r.scenario: r for r in results if 'upstream' in r.impl.lower()}

    # Raw benchmarks table
    raw_scenarios = sorted(set(
        r.scenario for r in results
        if r.test_type == 'raw'
    ))

    if raw_scenarios:
        lines.append("### Raw Benchmarks (localhost)\n")
        lines.append("| Scenario | noq | upstream | Delta | CPU (avg/max) |")
        lines.append("|----------|------------|----------|-------|---------------|")

        for scenario in raw_scenarios:
            iroh = iroh_results.get(scenario)
            upstream = upstream_results.get(scenario)

            if iroh:
                iroh_dl = f"{iroh.download_mbps:.1f} Mbps"
                cpu = f"{iroh.cpu_avg:.1f}% / {iroh.cpu_max:.1f}%"
            else:
                iroh_dl = "N/A"
                cpu = "N/A"

            if upstream:
                upstream_dl = f"{upstream.download_mbps:.1f} Mbps"
            else:
                upstream_dl = "N/A"

            if iroh and upstream:
                delta = format_delta(iroh.download_mbps, upstream.download_mbps)
            else:
                delta = "N/A"

            lines.append(f"| {scenario} | {iroh_dl} | {upstream_dl} | {delta} | {cpu} |")

        lines.append("")

    # Netsim benchmarks table
    netsim_scenarios = sorted(set(
        r.scenario for r in results
        if r.test_type == 'netsim'
    ))

    if netsim_scenarios:
        lines.append("### Netsim Benchmarks (network simulation)\n")
        lines.append("| Condition | noq | upstream | Delta |")
        lines.append("|-----------|------------|----------|-------|")

        for scenario in netsim_scenarios:
            iroh = iroh_results.get(scenario)
            upstream = upstream_results.get(scenario)
            condition = scenario.replace('netsim-', '')

            if iroh:
                iroh_dl = f"{iroh.download_mbps:.1f} Mbps"
            else:
                iroh_dl = "N/A"

            if upstream:
                upstream_dl = f"{upstream.download_mbps:.1f} Mbps"
            else:
                upstream_dl = "N/A"

            if iroh and upstream:
                delta = format_delta(iroh.download_mbps, upstream.download_mbps)
            else:
                delta = "N/A"

            lines.append(f"| {condition} | {iroh_dl} | {upstream_dl} | {delta} |")

        lines.append("")

    # Summary
    if iroh_results and upstream_results:
        common = set(iroh_results.keys()) & set(upstream_results.keys())
        if common:
            total_iroh = sum(iroh_results[s].download_mbps for s in common)
            total_upstream = sum(upstream_results[s].download_mbps for s in common)
            avg_delta = ((total_iroh - total_upstream) / total_upstream) * 100 if total_upstream else 0

            lines.append("### Summary\n")
            if avg_delta > 5:
                lines.append(f"noq is **{avg_delta:.1f}% faster** on average")
            elif avg_delta < -5:
                lines.append(f"noq is **{abs(avg_delta):.1f}% slower** on average")
            else:
                lines.append(f"Performance is roughly equivalent (delta: {avg_delta:+.1f}%)")

    if not results:
        lines.append("*No results available*")

    return "\n".join(lines)


def main():
    if len(sys.argv) < 3:
        print("Usage: compare_results.py <raw_results_dir> <netsim_results_dir>", file=sys.stderr)
        sys.exit(1)

    raw_dir = Path(sys.argv[1])
    netsim_dir = Path(sys.argv[2])

    results = collect_results(raw_dir, netsim_dir)
    print(generate_markdown(results))


if __name__ == '__main__':
    main()
