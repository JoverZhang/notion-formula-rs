import type { FormulaSchema, ValueType } from "../formula/client";
import { FORMULA_IDS, type FormulaId } from "./types";

export const PROPERTY_SCHEMA = [
  { id: "Title", ty: "String" },
  { id: "Text", ty: "String" },
  { id: "Number", ty: "Number" },
  { id: "Select", ty: "String" },
  { id: "Date", ty: "Date" },
  { id: "Relation", ty: { List: "String" } },
] as const satisfies readonly { id: string; ty: ValueType }[];

export type InputPropertyId = (typeof PROPERTY_SCHEMA)[number]["id"];

export const FORMULA_DEMOS: Record<FormulaId, { label: string; sample: string }> = {
  f1: { label: "Formula 1", sample: 'prop("Number") * 2' },
  f2: { label: "Formula 2", sample: 'prop("f1") + 1' },
};

export const DEMO_SCHEMA: FormulaSchema = {
  properties: [
    ...PROPERTY_SCHEMA.map(({ id, ty }) => ({ Input: { id, ty } })),
    ...FORMULA_IDS.map((id) => ({
      Formula: { id, expression: FORMULA_DEMOS[id].sample },
    })),
  ],
};
