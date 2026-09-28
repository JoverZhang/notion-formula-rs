#!/usr/bin/env python3
"""Reproduce the dated Notion empty-value experiment; write sanitized formula results to JSON."""

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

API = "https://api.notion.com/v1/"
VERSION = "2025-09-03"

# Every expression quoted in README.zh-CN.md appears here. The extra pairs make
# coincidentally equal results visible alongside the distinguishing results.
FORMULAS = [
    ("empty_value", "empty()"),
    ("empty_check", "empty(empty())"),
    ("empty_add", "empty() + 1"),
    ("number_value", 'prop("N")'),
    ("number_empty", 'empty(prop("N"))'),
    ("zero_empty", "empty(0)"),
    ("number_equal", 'prop("N") == 0'),
    ("zero_equal", "0 == 0"),
    ("number_join", 'join([1, prop("N"), 2], ",")'),
    ("zero_join", 'join([1, 0, 2], ",")'),
    ("number_unique", 'length(unique([prop("N"), 0]))'),
    ("zero_unique", "length(unique([0, 0]))"),
    ("number_add", 'prop("N") + 1'),
    ("zero_add", "0 + 1"),
    ("number_sum", 'sum([2, prop("N"), 4])'),
    ("zero_sum", "sum([2, 0, 4])"),
    ("number_mean", 'mean([2, prop("N"), 4])'),
    ("zero_mean", "mean([2, 0, 4])"),
    ("number_min", 'min([2, prop("N"), 4])'),
    ("zero_min", "min([2, 0, 4])"),
    ("text_equal", 'prop("T") == ""'),
    ("text_default_equal", '"" == ""'),
    ("text_unique", 'length(unique([prop("T"), ""]))'),
    ("text_default_unique", 'length(unique(["", ""]))'),
    ("boolean_equal", "if(false, true, empty()) == false"),
    ("boolean_default_equal", "false == false"),
    ("date_empty", 'empty(prop("D"))'),
    ("epoch_empty", "empty(fromTimestamp(0))"),
    ("date_timestamp", 'timestamp(prop("D"))'),
    ("epoch_timestamp", "timestamp(fromTimestamp(0))"),
    ("date_format", 'formatDate(prop("D"), "YYYY-MM-DD")'),
    ("epoch_format", 'formatDate(fromTimestamp(0), "YYYY-MM-DD")'),
    ("list_absent_length", "length(if(false, [1], empty()))"),
    ("list_empty_length", "length([])"),
    ("list_absent_flat", "length(flat([if(false, [1], empty())]))"),
    ("list_empty_flat", "length(flat([[]]))"),
    ("list_value", "[1, empty(), 2]"),
    ("list_length", "length([1, empty(), 2])"),
    ("list_at", "empty(at([1, empty(), 2], 1))"),
    ("list_join", 'join([1, empty(), 2], ",")'),
    ("list_map_index", 'join(map([1, empty(), 2], index), ",")'),
    ("list_map_empty", 'join(map([1, empty(), 2], empty(current)), ",")'),
    ("map_length", "length(map([1, 2, 3], if(current == 2, empty(), current)))"),
    ("map_join", 'join(map([1, 2, 3], if(current == 2, empty(), current)), ",")'),
    ("join_two_empty", 'join([empty(), empty()], ",")'),
    ("join_empty_list", 'join([], ",")'),
]
CONTEXTUAL_LIST = [
    ("typed_empty_map", "if(false, [], empty()).map(current)"),
    ("typed_empty_length", "if(false, [], empty()).map(current).length()"),
    ("typed_empty_join", 'if(false, [], empty()).join(",")'),
    ("nested_map_length", "[[], empty()].map(current.map(current)).length()"),
    ("nested_flat_length", "[[], empty()].map(current.map(current)).flat().length()"),
]
RUNTIME = [
    ("regex_test", 'test("abc", prop("Pattern"))'),
    ("regex_empty", 'empty(test("abc", prop("Pattern")))'),
    ("regex_format", 'format(test("abc", prop("Pattern")))'),
    ("regex_lazy", 'if(false, test("abc", prop("Pattern")), true)'),
    ("regex_literal_length", '[test("abc", prop("Pattern"))].length()'),
    ("regex_map_length", '[1].map(test("abc", prop("Pattern"))).length()'),
    ("regex_map_join", '[1].map(test("abc", prop("Pattern"))).join(",")'),
    ("empty_format", "format(empty())"),
    ("empty_check_runtime", "empty(empty())"),
    ("division", '1 / prop("Divisor")'),
    ("division_empty", 'empty(1 / prop("Divisor"))'),
    ("division_format", 'format(1 / prop("Divisor"))'),
    ("parse_date", 'parseDate(prop("Date Text"))'),
    ("parse_date_empty", 'empty(parseDate(prop("Date Text")))'),
]
REJECTED = [
    "map(empty(), current)", 'join(empty(), ",")',
    "empty().map(current)", "[empty()].map(current.map(current))",
]
CONTROL = {"number_equal", "number_unique", "number_mean", "date_empty", "date_timestamp"}


class ApiError(Exception):
    def __init__(self, status, payload):
        self.status = status
        self.payload = payload
        super().__init__(f"HTTP {status}: {payload.get('code', 'unknown_error')}")


def rich_text(value):
    return [{"type": "text", "text": {"content": value}}]


def request(token, method, path, body=None):
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {
        "Authorization": f"Bearer {token}",
        "Notion-Version": VERSION,
        "Content-Type": "application/json",
    }
    for attempt in range(5):
        time.sleep(0.36)  # Keep below Notion's average request rate.
        req = urllib.request.Request(API + path, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=45) as response:
                return json.load(response)
        except urllib.error.HTTPError as exc:
            raw = exc.read()
            try:
                payload = json.loads(raw)
            except ValueError:
                payload = {"message": raw.decode("utf-8", errors="replace")}
            if exc.code == 429 and attempt < 4:
                time.sleep(float(exc.headers.get("Retry-After", "1")))
                continue
            raise ApiError(exc.code, payload) from None
    raise RuntimeError("Notion API remained rate limited")


def formula_value(page, name):
    try:
        result = page["properties"][name]["formula"]
        return result, result[result["type"]]
    except (KeyError, TypeError) as exc:
        raise RuntimeError(f"Missing formula result for {name}") from exc


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    token = os.environ.get("NOTION_TOKEN")
    parent = os.environ.get("NOTION_PARENT_PAGE_ID")
    if not token or not parent:
        raise RuntimeError("Set NOTION_TOKEN and NOTION_PARENT_PAGE_ID before running")

    page = request(token, "POST", "pages", {
        "parent": {"page_id": parent},
        "properties": {"title": {"title": rich_text("Notion empty semantics probe")}},
    })
    database = request(token, "POST", "databases", {
        "parent": {"type": "page_id", "page_id": page["id"]},
        "title": rich_text("Empty values and defaults"),
        "initial_data_source": {"properties": {
            "Name": {"title": {}}, "N": {"number": {}},
            "T": {"rich_text": {}}, "D": {"date": {}},
            "Pattern": {"rich_text": {}}, "Date Text": {"rich_text": {}},
            "Divisor": {"number": {}},
        }},
    })
    source_id = database["data_sources"][0]["id"]
    names = {}
    for i, (label, expression) in enumerate(FORMULAS + CONTEXTUAL_LIST + RUNTIME):
        name = f"{i:02d}_{label}"
        request(token, "PATCH", f"data_sources/{source_id}", {
            "properties": {name: {"formula": {"expression": expression}}},
        })
        names[label] = name

    errors = []
    for expression in REJECTED:
        try:
            request(token, "PATCH", f"data_sources/{source_id}", {
                "properties": {"invalid_list_call": {"formula": {"expression": expression}}},
            })
        except ApiError as exc:
            if (exc.status, exc.payload.get("code"), exc.payload.get("message")) != (
                400, "validation_error", "Type error with formula"
            ):
                raise
            errors.append((expression, exc.payload["message"]))
        else:
            raise RuntimeError(f"Expected a type error for {expression}")

    def add_row(title, extra=None):
        properties = {"Name": {"title": rich_text(title)}}
        properties.update(extra or {})
        row = request(token, "POST", "pages", {
            "parent": {"type": "data_source_id", "data_source_id": source_id},
            "properties": properties,
        })
        return request(token, "GET", f"pages/{row['id']}")

    blank = add_row("Blank N, T and D")
    explicit = add_row("N = 0; D = Unix epoch", {
        "N": {"number": 0}, "D": {"date": {"start": "1970-01-01T00:00:00.000Z"}},
    })
    contextual = add_row("Contextual empty list")
    valid_pattern = add_row("Valid regex and numeric inputs", {
        "Pattern": {"rich_text": rich_text("a")},
        "Date Text": {"rich_text": rich_text("2026-09-28")},
        "Divisor": {"number": 2},
    })
    invalid_pattern = add_row("Invalid regex and empty-like inputs", {
        "Pattern": {"rich_text": rich_text("[")},
        "Date Text": {"rich_text": rich_text("not-a-date")},
        "Divisor": {"number": 0},
    })

    def results(row, cases):
        return [{"expression": expression, "formula": formula_value(row, names[label])[0]}
                for label, expression in cases]

    report = {
        "observed_on": str(datetime.now(timezone.utc).date()),
        "api_version": VERSION,
        "endpoint": "GET /v1/pages/{page_id}",
        "response_field": "properties[formula_name].formula",
        "rows": [
            {"name": "blank", "inputs": {"N": None, "T": None, "D": None},
             "results": results(blank, FORMULAS)},
            {"name": "control", "inputs": {"N": 0, "T": None,
                                            "D": "1970-01-01T00:00:00.000Z"},
             "results": results(explicit, [case for case in FORMULAS
                                           if case[0] in CONTROL])},
            {"name": "contextual_empty_list",
             "inputs": {"N": None, "T": None, "D": None},
             "results": results(contextual, CONTEXTUAL_LIST)},
            {"name": "valid_pattern",
             "inputs": {"Pattern": "a", "Date Text": "2026-09-28", "Divisor": 2},
             "results": results(valid_pattern, RUNTIME)},
            {"name": "invalid_pattern",
             "inputs": {"Pattern": "[", "Date Text": "not-a-date", "Divisor": 0},
             "results": results(invalid_pattern, RUNTIME)},
        ],
        "formula_creation_errors": [
            {"expression": expression, "status": 400,
             "code": "validation_error", "message": message}
            for expression, message in errors
        ],
    }
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Experiment results saved.")


if __name__ == "__main__":
    try:
        main()
    except (ApiError, RuntimeError, OSError, ValueError) as exc:
        message = str(exc) if isinstance(exc, (ApiError, RuntimeError)) else type(exc).__name__
        print(f"Probe failed: {message}", file=sys.stderr)
        sys.exit(1)
