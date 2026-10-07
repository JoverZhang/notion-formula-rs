import type {
  Column,
  ColumnData,
  EvaluateInput,
  RuntimeContext,
  Value,
  ValueType,
} from "@notion-formula/sdk";
import { PROPERTY_SCHEMA, type InputPropertyId } from "./context";
import { FORMULA_IDS } from "./types";

export type SampleRow = { id: string; values: Record<InputPropertyId, Value | null> };

export const SAMPLE_ROWS: SampleRow[] = [
  {
    id: "row-1",
    values: {
      Title: { String: "Prepare draft" },
      Text: { String: "Morning draft" },
      Number: { Number: 12 },
      Select: { String: "A" },
      Date: { Date: 1715644800000n },
      Relation: { List: [{ String: "North Star" }, { String: "Blueprint" }] },
    },
  },
  {
    id: "row-2",
    values: {
      Title: { String: "Meet client" },
      Text: { String: "Client check-in" },
      Number: { Number: 7 },
      Select: { String: "B" },
      Date: { Date: 1717286400000n },
      Relation: { List: [{ String: "Pulse" }] },
    },
  },
  {
    id: "row-3",
    values: {
      Title: { String: "Review quality" },
      Text: { String: "QA pass" },
      Number: { Number: 4 },
      Select: { String: "C" },
      Date: { Date: 1717891200000n },
      Relation: { List: [{ String: "Blueprint" }] },
    },
  },
  {
    id: "row-4",
    values: {
      Title: { String: "Finish report" },
      Text: { String: "Wrap report" },
      Number: { Number: 18 },
      Select: { String: "A" },
      Date: { Date: 1718496000000n },
      Relation: { List: [{ String: "North Star" }, { String: "Pulse" }] },
    },
  },
];

function columnData<T>(
  values: (Value | null)[],
  unwrap: (value: Value) => T,
  empty: T,
): ColumnData<T> {
  return {
    values: values.map((value) => (value === null ? empty : unwrap(value))),
    validity: values.map((value) => value !== null),
  };
}

function inputColumn(ty: ValueType, values: (Value | null)[]): Column {
  const wrongType = (): never => {
    throw new Error("Sample value does not match input schema");
  };
  if (ty === "String") {
    return {
      String: columnData(values, (value) => ("String" in value ? value.String : wrongType()), ""),
    };
  }
  if (ty === "Number") {
    return {
      Number: columnData(values, (value) => ("Number" in value ? value.Number : wrongType()), 0),
    };
  }
  if (ty === "Date") {
    return {
      Date: columnData(values, (value) => ("Date" in value ? value.Date : wrongType()), 0n),
    };
  }
  if (typeof ty === "object" && "List" in ty) {
    return {
      List: columnData(values, (value) => ("List" in value ? value.List : wrongType()), []),
    };
  }
  throw new Error("Unsupported demo input type");
}

export function buildEvaluateInput(runtime: RuntimeContext, rows = SAMPLE_ROWS): EvaluateInput {
  return {
    row_ids: rows.map((row) => row.id),
    columns: new Map(
      PROPERTY_SCHEMA.map(({ id, ty }) => [
        id,
        inputColumn(
          ty,
          rows.map((row) => row.values[id]),
        ),
      ]),
    ),
    runtime: { ...runtime },
    formula_ids: [...FORMULA_IDS],
  };
}
