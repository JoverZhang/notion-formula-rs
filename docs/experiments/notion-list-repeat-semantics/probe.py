#!/usr/bin/env python3
"""Measure Notion list nulls and repeat counts; record sanitized, dated evidence."""

import argparse
import getpass
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

API = "https://api.notion.com/v1/"
VERSION = "2025-09-03"


class ApiError(Exception):
    def __init__(self, status, payload):
        self.status = status
        self.payload = payload
        super().__init__(f"HTTP {status}: {payload.get('code', 'unknown_error')}")


def rich_text(text):
    return [{"type": "text", "text": {"content": text}}]


def request(token, method, path, body=None):
    data = None if body is None else json.dumps(body, allow_nan=False).encode()
    headers = {
        "Authorization": f"Bearer {token}",
        "Notion-Version": VERSION,
        "Content-Type": "application/json",
    }
    for attempt in range(5):
        time.sleep(0.4)
        req = urllib.request.Request(API + path, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=45) as response:
                return json.load(response)
        except urllib.error.HTTPError as exc:
            try:
                payload = json.loads(exc.read())
            except ValueError:
                payload = {"code": "non_json_error"}
            if exc.code == 429 and attempt < 4:
                time.sleep(float(exc.headers.get("Retry-After", "1")))
                continue
            raise ApiError(exc.code, payload) from None
    raise RuntimeError("Notion API remained rate limited")


def sanitized_error(exc, token):
    message = str(exc.payload.get("message", "")).replace(token, "[credential]")
    message = re.sub(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}",
                     "[resource_id]", message)
    return {"status": exc.status, "code": exc.payload.get("code"), "message": message}


def find_experiment_parent(token):
    candidates = []
    cursor = None
    while True:
        body = {"query": "Notion empty semantics probe", "page_size": 100,
                "filter": {"property": "object", "value": "page"}}
        if cursor:
            body["start_cursor"] = cursor
        result = request(token, "POST", "search", body)
        for page in result["results"]:
            title = "".join(part.get("plain_text", "")
                            for prop in page.get("properties", {}).values()
                            if prop.get("type") == "title" for part in prop["title"])
            if title == "Notion empty semantics probe" and not page.get("archived"):
                candidates.append(page)
        if not result.get("has_more"):
            break
        cursor = result["next_cursor"]
    if not candidates:
        raise RuntimeError("No prior experiment page found; set NOTION_PARENT_PAGE_ID")
    print(f"Found {len(candidates)} prior experiment page(s).", flush=True)
    return max(candidates, key=lambda page: page["created_time"])["id"]


def list_cases():
    cases = []

    def add(label, expression, category):
        cases.append({"label": label, "expression": expression, "category": category})

    arrays = [
        ("mixed", "[2, empty(), 4]"),
        ("zero_control", "[2, 0, 4]"),
        ("all_null", "[empty(), empty()]"),
        ("empty_list", "[]"),
        ("negative", "[empty(), -2, -4]"),
        ("property_null", '[2, prop("N"), 4]'),
    ]
    for function in ["sum", "mean", "median", "min", "max"]:
        for label, array in arrays:
            expression = f"{function}({array})"
            add(f"{function}_{label}", expression, "aggregate")
            if label in {"all_null", "empty_list"}:
                add(f"{function}_{label}_format", f"format({expression})", "aggregate_observer")
        add(f"{function}_variadic", f"{function}(2, empty(), 4)", "aggregate")
        add(f"{function}_null_argument", f"{function}(empty())", "aggregate")

    lists = [
        ("sort_middle", "sort([2, empty(), 1])", "sort"),
        ("sort_head", "sort([empty(), 2, 1])", "sort"),
        ("sort_tail", "sort([2, 1, empty()])", "sort"),
        ("sort_zeros", "sort([1, empty(), 0, -1, empty()])", "sort"),
        ("sort_all_null", "sort([empty(), empty()])", "sort"),
        ("sort_text", 'sort(["b", empty(), "a"])', "sort"),
        ("flat_scalar", "flat([1, empty(), 2])", "flat"),
        ("flat_nested", "flat([[1, empty()], [2]])", "flat"),
        ("flat_inner_null", "flat([[empty()], []])", "flat"),
        ("flat_outer_null", "flat([[], empty(), [2]])", "flat"),
        ("flat_typed_null", "flat([[], if(false, [1], empty()), [2]])", "flat"),
        ("flat_depth", "flat([[[]]])", "flat"),
    ]
    for label, expression, category in lists:
        add(label + "_length", f"length({expression})", category)
        add(label + "_join", f'join({expression}, "|")', category)
        add(label + "_marked", f'join(map({expression}, "<" + format(current) + ">"), "|")', category)
    add("flat_depth_element_length", 'join(map(flat([[[]]]), length(current)), "|")', "flat")
    failure = 'if(test("abc", prop("Pattern")), 1, 1)'
    for label, expression in [
        ("invalid_regex", 'test("abc", prop("Pattern"))'),
        ("aggregate_failure", f"sum([2, {failure}, 4])"),
        ("aggregate_failure_empty", f"empty(sum([2, {failure}, 4]))"),
        ("aggregate_failure_format", f"format(sum([2, {failure}, 4]))"),
    ]:
        add(label, expression, "failure_control")
    for label, count in [("negative", "-1"), ("fraction", "2.9"),
                         ("nan", "0 / 0"), ("infinity", "1 / 0"),
                         ("negative_infinity", "-1 / 0"), ("null", "empty()")]:
        expression = f'repeat("ab", {count})'
        for suffix, observed in [("length", f"length({expression})"),
                                 ("empty", f"empty({expression})"),
                                 ("preview", f"substring({expression}, 0, 12)")]:
            add(f"repeat_literal_{label}_{suffix}", observed, "repeat_literal")
        add(f"repeat_literal_{label}_count_format", f"format({count})", "repeat_count_control")
    return cases


def repeat_cases():
    expression = 'repeat(prop("Text"), prop("Count"))'
    return [
        {"label": label, "expression": formula, "category": "repeat_property"}
        for label, formula in [
            ("length", f"length({expression})"),
            ("empty", f"empty({expression})"),
            ("preview", f"substring({expression}, 0, 12)"),
            ("count_format", 'format(prop("Count"))'),
            ("text_length", 'length(prop("Text"))'),
        ]
    ]


def repeat_rows():
    counts = [-2, -1.9, -1, -0.9, -0.1, 0, 0.1, 0.9, 1, 1.1, 1.9, 2.9,
              10, 100, 999, 1000, 1001, 9999, 10000, 10001, 100000, 1000000, 1000001]
    rows = [{"name": f"count_{count}", "inputs": {"Count": count, "Text": "ab"}}
            for count in counts]
    rows.extend([
        {"name": "missing_count", "inputs": {"Count": None, "Text": "ab"}},
        {"name": "empty_text", "inputs": {"Count": 3, "Text": ""}},
        {"name": "empty_text_negative", "inputs": {"Count": -1, "Text": ""}},
        {"name": "empty_text_large", "inputs": {"Count": 1000001, "Text": ""}},
    ])
    return rows


def run_group(token, parent, name, cases, rows, report, save):
    database = request(token, "POST", "databases", {
        "parent": {"type": "page_id", "page_id": parent},
        "title": rich_text(name),
        "initial_data_source": {"properties": {
            "Name": {"title": {}}, "N": {"number": {}},
            "Count": {"number": {}}, "Text": {"rich_text": {}},
            "Pattern": {"rich_text": {}},
        }},
    })
    source = database["data_sources"][0]["id"]
    group = {"name": name, "formula_creation_errors": [], "rows": []}
    report["groups"].append(group)
    installed = []
    named_cases = [(f"p{index:03d}_{case['label']}", case) for index, case in enumerate(cases)]

    def install(batch):
        request(token, "PATCH", f"data_sources/{source}", {
            "properties": {name: {"formula": {"expression": case["expression"]}}
                           for name, case in batch},
        })

    # Batch independent definitions; retry validation failures individually so
    # a rejected expression cannot hide results from the other cases.
    for start in range(0, len(named_cases), 10):
        batch = named_cases[start:start + 10]
        try:
            install(batch)
        except ApiError as exc:
            if exc.status != 400 or exc.payload.get("code") != "validation_error":
                raise
            for named_case in batch:
                try:
                    install([named_case])
                except ApiError as single_error:
                    if single_error.status != 400 or single_error.payload.get("code") != "validation_error":
                        raise
                    group["formula_creation_errors"].append({**named_case[1], **sanitized_error(single_error, token)})
                else:
                    installed.append(named_case)
        else:
            installed.extend(batch)
        save()
        print(f"{name}: checked {min(start + 10, len(cases))}/{len(cases)} formulas.", flush=True)
    for row in rows:
        properties = {"Name": {"title": rich_text(row["name"])}}
        for key, value in row["inputs"].items():
            properties[key] = ({"number": value} if key in {"N", "Count"}
                               else {"rich_text": rich_text(value) if value else []})
        page = request(token, "POST", "pages", {
            "parent": {"type": "data_source_id", "data_source_id": source},
            "properties": properties,
        })
        observations = []
        # Preserve both reads rather than silently selecting a later answer.
        for reading in range(2):
            observed = request(token, "GET", f"pages/{page['id']}")
            results = []
            for property_name, case in installed:
                formula = observed["properties"][property_name].get("formula")
                if formula is None:
                    raise RuntimeError(f"Missing formula response for {case['label']}")
                results.append({**case, "formula": formula})
            observations.append(results)
        stored_inputs = {}
        for key in row["inputs"]:
            stored = observed["properties"][key]
            if stored["type"] == "number":
                stored_inputs[key] = stored["number"]
            else:
                stored_inputs[key] = "".join(part["plain_text"] for part in stored["rich_text"])
        group["rows"].append({**row, "stored_inputs": stored_inputs, "results": observations[0],
                              "repeat_read_matches": observations[0] == observations[1]})
        if observations[0] != observations[1]:
            group["rows"][-1]["second_read"] = observations[1]
        save()
        print(f"{name}: recorded {row['name']}.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--phase", choices=["all", "lists", "repeat"], default="all")
    parser.add_argument("--cases", type=Path, nargs="+", help="JSON files with name, cases and rows for follow-up groups")
    args = parser.parse_args()
    if args.output.exists():
        raise RuntimeError("Output already exists; choose a new file to preserve prior evidence")
    token = os.environ.get("NOTION_TOKEN") or getpass.getpass("Notion token: ")
    parent = os.environ.get("NOTION_PARENT_PAGE_ID") or find_experiment_parent(token)
    today = str(datetime.now(timezone.utc).date())
    page = request(token, "POST", "pages", {
        "parent": {"page_id": parent},
        "properties": {"title": {"title": rich_text(f"List nulls and repeat probe {today}")}},
    })
    report = {"observed_on": today, "api_version": VERSION,
              "endpoint": "GET /v1/pages/{page_id}",
              "response_field": "properties[formula_name].formula",
              "groups": [], "complete": False}

    def save():
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + "\n")

    if args.cases:
        for case_file in args.cases:
            group = json.loads(case_file.read_text())
            run_group(token, page["id"], group["name"], group["cases"], group["rows"], report, save)
    else:
        if args.phase in {"all", "lists"}:
            run_group(token, page["id"], "List nulls and literal counts", list_cases(), [
                {"name": "blank_number_invalid_regex", "inputs": {"N": None, "Pattern": "["}},
                {"name": "number_and_valid_regex", "inputs": {"N": 7, "Pattern": "a"}},
            ], report, save)
        if args.phase in {"all", "repeat"}:
            run_group(token, page["id"], "Repeat property counts", repeat_cases(), repeat_rows(), report, save)
    report["complete"] = True
    save()
    print("Experiment complete; sanitized results saved.", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (ApiError, RuntimeError, OSError, ValueError) as exc:
        safe = str(exc) if isinstance(exc, (ApiError, RuntimeError)) else type(exc).__name__
        print(f"Probe failed: {safe}", file=sys.stderr)
        sys.exit(1)
