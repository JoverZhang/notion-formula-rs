import type { AnalyzerConfig, Property } from "../analyzer/generated/wasm_dto";

export const PROPERTY_SCHEMA: Property[] = [
  { name: "Title", type: "String" },
  { name: "Text", type: "String" },
  { name: "Number", type: "Number" },
  { name: "Select", type: "String" },
  { name: "Date", type: "Date" },
  { name: "Relation", type: { List: "String" } },
];

export const ANALYZER_CONFIG: AnalyzerConfig = {
  properties: PROPERTY_SCHEMA,
  preferred_limit: null,
};
