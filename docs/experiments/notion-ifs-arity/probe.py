#!/usr/bin/env python3
"""Measure Notion ifs arity using the existing sanitized database probe helpers."""

import argparse
import getpass
import json
import os
import runpy
import sys
from datetime import datetime, timezone
from pathlib import Path

SHARED = runpy.run_path(str(
    Path(__file__).resolve().parent.parent / "notion-list-repeat-semantics" / "probe.py"
))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise RuntimeError("Output already exists; choose a new file to preserve prior evidence")
    groups = json.loads(Path(__file__).with_name("cases.json").read_text())
    token = os.environ.get("NOTION_TOKEN") or getpass.getpass("Notion token: ")
    parent = os.environ.get("NOTION_PARENT_PAGE_ID") or SHARED["find_experiment_parent"](token)
    today = str(datetime.now(timezone.utc).date())
    page = SHARED["request"](token, "POST", "pages", {
        "parent": {"page_id": parent},
        "properties": {"title": {"title": SHARED["rich_text"](f"Notion ifs arity probe {today}")}},
    })
    report = {
        "observed_on": today,
        "api_version": SHARED["VERSION"],
        "endpoint": "GET /v1/pages/{page_id}",
        "response_field": "properties[formula_name].formula",
        "groups": [],
        "complete": False,
    }

    def save():
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + "\n")

    save()
    for group in groups:
        SHARED["run_group"](
            token, page["id"], group["name"], group["cases"], group["rows"], report, save
        )
    report["complete"] = True
    save()
    print("Experiment complete; sanitized results saved.", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (SHARED["ApiError"], RuntimeError, OSError, ValueError) as exc:
        safe = str(exc) if isinstance(exc, (SHARED["ApiError"], RuntimeError)) else type(exc).__name__
        print(f"Probe failed: {safe}", file=sys.stderr)
        sys.exit(1)
