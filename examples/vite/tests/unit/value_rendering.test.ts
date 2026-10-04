import { describe, expect, it } from "vitest";
import type { Column, Value } from "../../src/formula/client";
import {
  columnValue,
  formatDateValue,
  formatRuntimeError,
  formatValue,
  formatValueType,
} from "../../src/model/values";

describe("engine value presentation", () => {
  it.each([
    [null, "null"],
    [{ Number: NaN }, "NaN"],
    [{ Number: Infinity }, "Infinity"],
    [{ Number: -Infinity }, "-Infinity"],
    [{ Number: -0 }, "-0"],
    [{ Number: 0 }, "0"],
    [{ String: "<img src=x>" }, "<img src=x>"],
    [{ Boolean: false }, "false"],
    [
      { List: [{ Number: -0 }, null, { List: [{ Boolean: true }, { String: "hello" }] }] },
      "[-0, null, [true, hello]]",
    ],
  ] satisfies [Value | null, string][])("formats %j", (value, expected) => {
    expect(formatValue(value)).toBe(expected);
  });

  it("converts representable dates and retains exact timestamps beyond JS Date limits", () => {
    expect(formatDateValue(0n)).toBe("1970-01-01");
    expect(formatDateValue(1n)).toBe("1970-01-01T00:00:00.001Z");
    expect(formatDateValue(-1n)).toBe("1969-12-31T23:59:59.999Z");
    expect(formatDateValue(8640000000000000n)).toBe("+275760-09-13");
    expect(formatDateValue(-8640000000000000n)).toBe("-271821-04-20");
    for (const timestamp of [
      8640000000000001n,
      -8640000000000001n,
      9007199254740993n,
      -9007199254740993n,
    ]) {
      expect(formatDateValue(timestamp)).toBe(timestamp.toString());
    }
  });

  it("reads typed columns through validity rather than the placeholder value", () => {
    const columns: Column[] = [
      { Number: { values: [-0, 42], validity: [true, false] } },
      { String: { values: ["hello", "ignored"], validity: [true, false] } },
      { Boolean: { values: [false, true], validity: [true, false] } },
      { Date: { values: [0n, 1n], validity: [true, false] } },
      { List: { values: [[null, { Number: 3 }], []], validity: [true, false] } },
      { Union: { values: [{ Number: NaN }, { String: "ignored" }], validity: [true, false] } },
    ];
    expect(columns.map((column) => formatValue(columnValue(column, 0)))).toEqual([
      "-0",
      "hello",
      "false",
      "1970-01-01",
      "[null, 3]",
      "NaN",
    ]);
    expect(columns.map((column) => columnValue(column, 1))).toEqual(Array(6).fill(null));
  });

  it("displays recursive schema types and readable runtime errors", () => {
    expect(formatValueType({ List: { Union: ["Number", "Date", { List: "String" }] } })).toBe(
      "list<number | date | list<string>>",
    );
    expect(formatValueType("Unknown")).toBe("unknown");
    expect(formatRuntimeError({ InvalidValueType: { expected: "Number", actual: "String" } })).toBe(
      "Expected number, received string",
    );
    expect(
      formatRuntimeError({
        InvalidValue: { actual: { Date: 9007199254740993n }, constraint: "in range" },
      }),
    ).toBe("Invalid value 9007199254740993: in range");
    expect(formatRuntimeError({ InvalidRegex: { pattern: "[", detail: "unclosed class" } })).toBe(
      "Invalid regular expression [: unclosed class",
    );
    expect(formatRuntimeError({ InvalidDateText: { text: "tomorrow?" } })).toBe(
      "Invalid date text: tomorrow?",
    );
    expect(formatRuntimeError("DateOutOfRange")).toBe("Date is out of range");
  });
});
