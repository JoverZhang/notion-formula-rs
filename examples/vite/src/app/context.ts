export const PROPERTY_SCHEMA = [
  { name: "Title", type: "String" },
  { name: "Text", type: "String" },
  { name: "Number", type: "Number" },
  { name: "Select", type: "String" },
  { name: "Date", type: "Date" },
  { name: "Relation", type: "Unknown" },
] as const;

export const FUNCTION_SCHEMA = [
  {
    name: "if",
    params: [
      { name: "cond", ty: "Boolean", optional: false },
      { name: "then", ty: "Unknown", optional: false },
      { name: "else", ty: "Unknown", optional: false },
    ],
    ret: "Unknown",
    detail: "if(condition, then, else)",
  },
  {
    name: "sum",
    params: [
      { name: "a", ty: "Number", optional: false },
      { name: "b", ty: "Number", optional: true },
      { name: "c", ty: "Number", optional: true },
    ],
    ret: "Number",
    detail: "sum(number, ...)",
  },
  {
    name: "prop",
    params: [{ name: "name", ty: "String", optional: false }],
    ret: "Unknown",
    detail: 'prop("Property")',
  },
  {
    name: "formatDate",
    params: [
      { name: "date", ty: "Date", optional: false },
      { name: "format", ty: "String", optional: false },
    ],
    ret: "String",
    detail: "formatDate(date, format)",
  },
] as const;

export const CONTEXT_JSON = JSON.stringify({
  properties: PROPERTY_SCHEMA,
  functions: FUNCTION_SCHEMA,
});
