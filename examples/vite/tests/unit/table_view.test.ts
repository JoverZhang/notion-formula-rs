// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import type { EvaluateResult, FormulaOutput } from "../../src/formula/client";
import { createFormulaTableView } from "../../src/ui/table_view";

const numbers = (values = [24, 14, 8, 36]): FormulaOutput => ({
  output_type: "Number",
  column: { Number: { values, validity: [true, true, true, true] } },
  errors: [],
});
function output(f1: FormulaOutput, f2: FormulaOutput = numbers([25, 15, 9, 37])): EvaluateResult {
  return {
    formulas: new Map([
      ["Formula 1", { Ok: f1 }],
      ["Formula 2", { Ok: f2 }],
    ]),
  };
}
function cells(id = "Formula 1") {
  return [
    ...document.querySelectorAll<HTMLTableCellElement>(
      `[data-testid="formula-cell"][data-formula-id="${id}"]`,
    ),
  ];
}
function mount() {
  const view = createFormulaTableView();
  view.mount(document.body);
  return view;
}

describe("formula result table", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("shows the complete sample and replaces pending cells with saved formula outputs", () => {
    const view = mount();
    expect(cells().map((cell) => cell.textContent)).toEqual(Array(4).fill("<pending>"));
    expect([...document.querySelectorAll("thead th")].map((cell) => cell.textContent)).toEqual([
      "Title",
      "Text",
      "Number",
      "Select",
      "Date",
      "Relation",
      "Formula 1",
      "Formula 2",
    ]);
    expect(
      document.querySelector('[data-row-id="row-1"] [data-property-id="Date"]')?.textContent,
    ).toBe("2024-05-14");
    expect(
      document.querySelector('[data-row-id="row-1"] [data-property-id="Relation"]')?.textContent,
    ).toBe("[North Star, Blueprint]");
    view.update(output(numbers()));
    expect(cells().map((cell) => cell.textContent)).toEqual(["24", "14", "8", "36"]);
    expect(cells("Formula 2").map((cell) => cell.textContent)).toEqual(["25", "15", "9", "37"]);
    expect(
      document.querySelector('th[data-formula-id="Formula 1"]')?.getAttribute("data-output-type"),
    ).toBe("number");
    expect(
      document.querySelector('th[data-formula-id="Formula 1"] .table-output-type')?.textContent,
    ).toBe(" (number)");
  });

  it("distinguishes a failed initial evaluation from pending initialization", () => {
    const view = mount();
    view.update(null, "Evaluation failed");
    for (const id of ["Formula 1", "Formula 2"]) {
      expect(cells(id).map((cell) => cell.textContent)).toEqual(Array(4).fill("Unavailable"));
      expect(cells(id).map((cell) => cell.title)).toEqual(Array(4).fill("Evaluation failed"));
    }
    expect(document.querySelector('[data-testid="table-error"]')?.textContent).toBe(
      "Evaluation failed",
    );
    view.update(null);
    expect(cells().map((cell) => cell.textContent)).toEqual(Array(4).fill("<pending>"));
    expect(cells().map((cell) => cell.title)).toEqual(Array(4).fill(""));
    view.update(output(numbers()));
    expect(cells().map((cell) => cell.textContent)).toEqual(["24", "14", "8", "36"]);
  });

  it("distinguishes ordinary null from row errors and preserves every error origin", () => {
    const view = mount();
    const f1 = numbers();
    f1.column = { Number: { values: [0, 0, 8, 36], validity: [false, false, true, true] } };
    f1.errors = [
      { row_index: 1, origin_formula_id: "source-a", error: "DateOutOfRange" },
      {
        row_index: 1,
        origin_formula_id: "source-b",
        error: { InvalidDateText: { text: "bad date" } },
      },
    ];
    view.update(output(f1));
    expect(cells().map((cell) => cell.textContent)).toEqual(["null", "Error", "8", "36"]);
    expect(cells()[0].classList.contains("is-error")).toBe(false);
    expect(cells()[0].title).toBe("");
    expect(cells()[1].classList.contains("is-error")).toBe(true);
    expect(cells()[1].title).toBe(
      "source-a: Date is out of range\nsource-b: Invalid date text: bad date",
    );
    expect(cells()[1].dataset.errorOrigins).toBe("source-a, source-b");
    expect(cells("Formula 2").map((cell) => cell.textContent)).toEqual(["25", "15", "9", "37"]);
    view.update(output(numbers()));
    expect(cells()[1].title).toBe("");
    expect(cells()[1].dataset.errorOrigins).toBeUndefined();
    expect(cells()[1].classList.contains("is-error")).toBe(false);
  });

  it("uses engine output_type even for all-null columns and reports Not ready", () => {
    const view = mount();
    view.update({
      formulas: new Map([
        [
          "Formula 1",
          {
            Ok: {
              output_type: { Union: ["Number", "String"] },
              column: {
                Union: {
                  values: Array.from({ length: 4 }, () => ({ Number: 0 })),
                  validity: [false, false, false, false],
                },
              },
              errors: [],
            },
          },
        ],
        ["Formula 2", { Err: "NotReady" }],
      ]),
    });
    expect(cells().map((cell) => cell.textContent)).toEqual(Array(4).fill("null"));
    expect(
      document.querySelector('th[data-formula-id="Formula 1"]')?.getAttribute("data-output-type"),
    ).toBe("number | string");
    expect(cells("Formula 2").map((cell) => cell.textContent)).toEqual(Array(4).fill("Not ready"));
    expect(
      document.querySelector('th[data-formula-id="Formula 2"]')?.getAttribute("data-output-type"),
    ).toBeNull();
  });

  it("keeps successful rows available and inserts strings and errors as text", () => {
    const view = mount();
    view.update(
      output({
        output_type: "String",
        column: {
          String: {
            values: ["<img src=x>", "<script>bad()</script>", "ok", "done"],
            validity: [true, true, true, true],
          },
        },
        errors: [],
      }),
      "<b>worker failed</b>",
    );
    expect(cells()[0].textContent).toBe("<img src=x>");
    expect(cells()[1].textContent).toBe("<script>bad()</script>");
    expect(document.querySelector("img, script, b")).toBeNull();
    const error = document.querySelector('[data-testid="table-error"]')!;
    expect(error.textContent).toBe("<b>worker failed</b>");
    expect(error.classList.contains("hidden")).toBe(false);
    view.update(output(numbers()));
    expect(error.classList.contains("hidden")).toBe(true);
  });

  it("renders special numbers, nested lists, nulls and exact date timestamps in cells", () => {
    const view = mount();
    view.update(
      output(numbers([NaN, Infinity, -Infinity, -0]), {
        output_type: { List: { Union: ["Date", "Boolean", "String", { List: "Number" }] } },
        column: {
          List: {
            values: [
              [{ Date: 9007199254740993n }, null],
              [{ Boolean: false }, { List: [{ Number: -0 }, null] }],
              [{ Date: 0n }, { String: "<b>hello</b>" }],
              [],
            ],
            validity: [true, true, true, true],
          },
        },
        errors: [],
      }),
    );
    expect(cells().map((cell) => cell.textContent)).toEqual(["NaN", "Infinity", "-Infinity", "-0"]);
    expect(cells("Formula 2").map((cell) => cell.textContent)).toEqual([
      "[9007199254740993, null]",
      "[false, [-0, null]]",
      "[1970-01-01, <b>hello</b>]",
      "[]",
    ]);
    expect(document.querySelector("b")).toBeNull();
  });
});
