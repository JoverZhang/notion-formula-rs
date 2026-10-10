import type { Column, RuntimeError, Value, ValueType } from "@notion-formula/sdk";

export function formatValueType(ty: ValueType): string {
  if (typeof ty === "string") return ty.toLowerCase();
  if ("List" in ty) return `list<${formatValueType(ty.List)}>`;
  return ty.Union.map(formatValueType).join(" | ");
}

export function formatDateValue(value: bigint, includeTime?: boolean): string {
  // Both limits matter before converting an exact engine timestamp to a JS number.
  const dateLimit = 8640000000000000n;
  const safeLimit = BigInt(Number.MAX_SAFE_INTEGER);
  if (value < -dateLimit || value > dateLimit || value < -safeLimit || value > safeLimit) {
    return value.toString();
  }
  const iso = new Date(Number(value)).toISOString();
  return includeTime === false || (includeTime === undefined && iso.endsWith("T00:00:00.000Z"))
    ? iso.slice(0, iso.indexOf("T"))
    : iso;
}

export function formatValue(value: Value | null): string {
  if (value === null) return "null";
  if ("Number" in value) return Object.is(value.Number, -0) ? "-0" : String(value.Number);
  if ("String" in value) return value.String;
  if ("Boolean" in value) return String(value.Boolean);
  if ("Date" in value) return formatDateValue(value.Date);
  if ("DateValue" in value) {
    const date = value.DateValue;
    const endpoint = (timestamp: bigint) => formatDateValue(timestamp, date.include_time);
    const start = endpoint(date.start);
    return date.end === null ? start : `${start} → ${endpoint(date.end)}`;
  }
  return `[${value.List.map(formatValue).join(", ")}]`;
}

export function columnValue(column: Column, rowIndex: number): Value | null {
  if ("Number" in column)
    return column.Number.validity[rowIndex] ? { Number: column.Number.values[rowIndex] } : null;
  if ("String" in column)
    return column.String.validity[rowIndex] ? { String: column.String.values[rowIndex] } : null;
  if ("Boolean" in column)
    return column.Boolean.validity[rowIndex] ? { Boolean: column.Boolean.values[rowIndex] } : null;
  if ("Date" in column)
    return column.Date.validity[rowIndex] ? { Date: column.Date.values[rowIndex] } : null;
  if ("DateValue" in column)
    return column.DateValue.validity[rowIndex]
      ? { DateValue: column.DateValue.values[rowIndex] }
      : null;
  if ("List" in column)
    return column.List.validity[rowIndex] ? { List: column.List.values[rowIndex] } : null;
  return column.Union.validity[rowIndex] ? column.Union.values[rowIndex] : null;
}

export function formatRuntimeError(error: RuntimeError): string {
  if (error === "DateOutOfRange") return "Date is out of range";
  if ("InvalidValueType" in error) {
    return `Expected ${formatValueType(error.InvalidValueType.expected)}, received ${formatValueType(error.InvalidValueType.actual)}`;
  }
  if ("InvalidValue" in error)
    return `Invalid value ${formatValue(error.InvalidValue.actual)}: ${error.InvalidValue.constraint}`;
  if ("InvalidRegex" in error)
    return `Invalid regular expression ${error.InvalidRegex.pattern}: ${error.InvalidRegex.detail}`;
  return `Invalid date text: ${error.InvalidDateText.text}`;
}
