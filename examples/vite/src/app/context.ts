export const PROPERTY_SCHEMA = [
  { name: "Text", type: "String" },
  { name: "Number", type: "Number" },
  { name: "Select", type: "String" },
  { name: "Date", type: "Date" },
  { name: "Relation", type: "Unknown" },
] as const;

export const CONTEXT_JSON = JSON.stringify({ properties: PROPERTY_SCHEMA });
