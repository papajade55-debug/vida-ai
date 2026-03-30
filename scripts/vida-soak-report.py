#!/usr/bin/env python3

import json
import sys
from pathlib import Path


def main() -> int:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("/var/log/vida-ai/soak-samples.jsonl")
    if not path.exists():
        print(f"missing file: {path}", file=sys.stderr)
        return 1

    samples = []
    invalid_lines = 0
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            samples.append(json.loads(line))
        except json.JSONDecodeError:
            invalid_lines += 1
            continue

    if not samples:
        print(json.dumps({"sample_count": 0, "invalid_lines": invalid_lines}, ensure_ascii=True, indent=2))
        return 0

    health_codes = [int(sample.get("health_http_code", 0)) for sample in samples]
    times = [float(sample.get("health_time_total_sec", 0)) for sample in samples]
    rss_values = [int(sample.get("vida_rss_kb", 0)) for sample in samples]
    db_sizes = [int(sample.get("db_bytes", 0)) for sample in samples]
    error_counts = [int(sample.get("journal_errors_15m", 0)) for sample in samples]
    failures = [code for code in health_codes if code != 200]

    report = {
        "sample_count": len(samples),
        "invalid_lines": invalid_lines,
        "window_start": samples[0].get("ts_utc"),
        "window_end": samples[-1].get("ts_utc"),
        "health_failures": len(failures),
        "health_success_rate_pct": round(((len(samples) - len(failures)) / len(samples)) * 100, 2),
        "health_time_avg_sec": round(sum(times) / len(times), 4),
        "health_time_max_sec": round(max(times), 4),
        "vida_rss_kb_min": min(rss_values),
        "vida_rss_kb_max": max(rss_values),
        "db_bytes_min": min(db_sizes),
        "db_bytes_max": max(db_sizes),
        "journal_errors_15m_max": max(error_counts),
        "last_sample": samples[-1],
    }
    print(json.dumps(report, ensure_ascii=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
