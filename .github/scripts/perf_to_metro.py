#!/usr/bin/env python3
"""
Convert performance results to Metro format for dashboard ingestion.
"""

import argparse
import json
import os
import sys
from pathlib import Path
from datetime import datetime


def parse_quinn_perf_json(path: Path) -> dict | None:
    """Parse quinn-perf JSON output and extract metrics."""
    try:
        with open(path) as f:
            data = json.load(f)

        if not data.get('intervals'):
            return None

        # Calculate metrics from intervals
        total_send_bps = 0
        total_recv_bps = 0
        interval_count = 0

        for interval in data['intervals']:
            for stream in interval.get('streams', []):
                if stream.get('sender'):
                    total_send_bps += stream.get('bits_per_second', 0)
                else:
                    total_recv_bps += stream.get('bits_per_second', 0)
            interval_count += 1

        if interval_count == 0:
            return None

        return {
            'upload_mbps': (total_send_bps / interval_count) / 1_000_000,
            'download_mbps': (total_recv_bps / interval_count) / 1_000_000,
        }
    except (json.JSONDecodeError, KeyError, FileNotFoundError):
        return None


def parse_metadata_json(path: Path) -> dict | None:
    """Parse metadata.json for CPU stats."""
    try:
        with open(path) as f:
            return json.load(f)
    except (json.JSONDecodeError, FileNotFoundError):
        return None


def collect_metrics(raw_dir: Path, netsim_dir: Path, commit: str) -> list[dict]:
    """Collect all metrics and convert to Metro format."""
    metrics = []
    timestamp = datetime.utcnow().isoformat() + "Z"

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
                    base_tags = {
                        "implementation": impl,
                        "scenario": scenario,
                        "test_type": "raw",
                        "commit": commit[:7]
                    }

                    metrics.append({
                        "name": "quinn_perf_throughput_download_mbps",
                        "value": perf_data['download_mbps'],
                        "timestamp": timestamp,
                        "tags": base_tags.copy()
                    })

                    metrics.append({
                        "name": "quinn_perf_throughput_upload_mbps",
                        "value": perf_data['upload_mbps'],
                        "timestamp": timestamp,
                        "tags": base_tags.copy()
                    })

                    if metadata:
                        metrics.append({
                            "name": "quinn_perf_cpu_avg",
                            "value": metadata.get('cpu_avg', 0),
                            "timestamp": timestamp,
                            "tags": base_tags.copy()
                        })

                        metrics.append({
                            "name": "quinn_perf_cpu_max",
                            "value": metadata.get('cpu_max', 0),
                            "timestamp": timestamp,
                            "tags": base_tags.copy()
                        })

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
                    netsim_tags = {
                        "implementation": impl,
                        "scenario": f"netsim-{condition}",
                        "test_type": "netsim",
                        "network_condition": condition,
                        "commit": commit[:7]
                    }

                    metrics.append({
                        "name": "quinn_perf_throughput_download_mbps",
                        "value": perf_data['download_mbps'],
                        "timestamp": timestamp,
                        "tags": netsim_tags.copy()
                    })

                    metrics.append({
                        "name": "quinn_perf_throughput_upload_mbps",
                        "value": perf_data['upload_mbps'],
                        "timestamp": timestamp,
                        "tags": netsim_tags.copy()
                    })

    return metrics


def main():
    parser = argparse.ArgumentParser(description='Convert perf results to Metro format')
    parser.add_argument('raw_dir', type=Path, help='Directory with raw benchmark results')
    parser.add_argument('netsim_dir', type=Path, help='Directory with netsim results')
    parser.add_argument('--commit', required=True, help='Git commit SHA')
    parser.add_argument('--bucket', default='quic', help='Metro bucket name')

    args = parser.parse_args()

    metrics = collect_metrics(args.raw_dir, args.netsim_dir, args.commit)

    # Output in Metro format
    output = {
        "bucket": args.bucket,
        "metrics": metrics
    }

    print(json.dumps(output, indent=2))


if __name__ == '__main__':
    main()
