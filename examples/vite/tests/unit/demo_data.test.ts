import { describe, expect, it } from "vitest";
import { DEMO_SCHEMA, FORMULA_DEMOS, PROPERTY_SCHEMA } from "../../src/app/context";
import { buildEvaluateInput, SAMPLE_ROWS } from "../../src/app/data";
import type { ColumnData } from "../../src/formula/client";
import { columnValue, formatValue } from "../../src/model/values";

const runtime = { now: 1791043200000n, time_zone: "+08:00" };

describe("demo evaluate input", () => {
  it("supplies every schema input as a typed column with complete validity", () => {
    const input = buildEvaluateInput(runtime);
    expect(input.row_ids).toEqual(["row-1", "row-2", "row-3", "row-4"]);
    expect([...input.columns.keys()]).toEqual(PROPERTY_SCHEMA.map(({ id }) => id));
    expect(DEMO_SCHEMA.properties.filter((property) => "Input" in property)).toEqual(
      PROPERTY_SCHEMA.map((property) => ({ Input: property })),
    );
    for (const column of input.columns.values()) {
      const data = Object.values(column)[0] as ColumnData<unknown>;
      expect(data.values).toHaveLength(input.row_ids.length);
      expect(data.validity).toEqual([true, true, true, true]);
    }
    expect(input.columns.get("Number")).toEqual({
      Number: { values: [12, 7, 4, 18], validity: [true, true, true, true] },
    });
    expect(input.columns.get("Title")).toEqual({
      String: {
        values: ["Prepare draft", "Meet client", "Review quality", "Finish report"],
        validity: [true, true, true, true],
      },
    });
    expect(input.columns.get("Relation")).toEqual({
      List: {
        values: [
          [{ String: "North Star" }, { String: "Blueprint" }],
          [{ String: "Pulse" }],
          [{ String: "Blueprint" }],
          [{ String: "North Star" }, { String: "Pulse" }],
        ],
        validity: [true, true, true, true],
      },
    });
    expect(input.formula_ids).toEqual(["f1", "f2"]);
    expect(DEMO_SCHEMA.properties.filter((property) => "Formula" in property)).toEqual([
      { Formula: { id: "f1", expression: FORMULA_DEMOS.f1.sample } },
      { Formula: { id: "f2", expression: FORMULA_DEMOS.f2.sample } },
    ]);
    expect(input.runtime).toEqual(runtime);
    expect(input.runtime).not.toBe(runtime);
  });

  it("preserves UTC millisecond dates exactly", () => {
    const column = buildEvaluateInput(runtime).columns.get("Date")!;
    expect(column).toEqual({
      Date: {
        values: [1715644800000n, 1717286400000n, 1717891200000n, 1718496000000n],
        validity: [true, true, true, true],
      },
    });
    expect(SAMPLE_ROWS.map((_, rowIndex) => formatValue(columnValue(column, rowIndex)))).toEqual([
      "2024-05-14",
      "2024-06-02",
      "2024-06-09",
      "2024-06-16",
    ]);
  });

  it("uses validity for nullable dates and lists while preserving non-null special numbers", () => {
    const rows = structuredClone(SAMPLE_ROWS);
    rows[0].values.Date = null;
    rows[1].values.Relation = null;
    rows[2].values.Number = { Number: -0 };
    rows[3].values.Number = { Number: NaN };
    const input = buildEvaluateInput(runtime, rows);
    expect(input.columns.get("Date")).toEqual({
      Date: {
        values: [0n, 1717286400000n, 1717891200000n, 1718496000000n],
        validity: [false, true, true, true],
      },
    });
    expect(input.columns.get("Relation")).toMatchObject({
      List: { validity: [true, false, true, true] },
    });
    expect(columnValue(input.columns.get("Relation")!, 1)).toBeNull();
    expect(columnValue(input.columns.get("Date")!, 0)).toBeNull();
    expect(formatValue(columnValue(input.columns.get("Number")!, 2))).toBe("-0");
    expect(formatValue(columnValue(input.columns.get("Number")!, 3))).toBe("NaN");
    expect(SAMPLE_ROWS[0].values.Date).toEqual({ Date: 1715644800000n });
  });
});
