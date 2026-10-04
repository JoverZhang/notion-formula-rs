import { FORMULA_DEMOS, PROPERTY_SCHEMA } from "../app/context";
import { SAMPLE_ROWS } from "../app/data";
import { FORMULA_IDS, type FormulaId } from "../app/types";
import type { EvaluateResult } from "../formula/client";
import { columnValue, formatRuntimeError, formatValue, formatValueType } from "../model/values";

export function createFormulaTableView() {
  const root = document.createElement("div");
  root.className = "table-card";
  root.innerHTML = `
    <div>
      <h2 class="table-title">Tasks</h2>
      <p class="table-subtitle">Sample rows with saved formula outputs.</p>
      <p class="table-error hidden" role="status" data-testid="table-error"></p>
    </div>
    <div class="table-scroll"></div>
  `;

  const scroll = root.querySelector<HTMLDivElement>(".table-scroll")!;
  const errorMessage = root.querySelector<HTMLParagraphElement>(".table-error")!;
  const table = document.createElement("table");
  table.className = "notion-table";
  table.setAttribute("data-testid", "formula-table");
  scroll.appendChild(table);

  const head = table.createTHead().insertRow();
  for (const property of PROPERTY_SCHEMA) {
    const th = document.createElement("th");
    th.textContent = property.id;
    th.title = formatValueType(property.ty);
    th.dataset.propertyId = property.id;
    head.appendChild(th);
  }

  const formulaHeaders = new Map<FormulaId, HTMLTableCellElement>();
  const formulaCells = new Map<FormulaId, HTMLTableCellElement[]>(
    FORMULA_IDS.map((id) => [id, []]),
  );
  for (const id of FORMULA_IDS) {
    const th = document.createElement("th");
    th.textContent = FORMULA_DEMOS[id].label;
    th.dataset.formulaId = id;
    head.appendChild(th);
    formulaHeaders.set(id, th);
  }

  const body = table.createTBody();
  for (const row of SAMPLE_ROWS) {
    const tr = body.insertRow();
    tr.setAttribute("data-row-id", row.id);
    for (const property of PROPERTY_SCHEMA) {
      const td = tr.insertCell();
      td.dataset.propertyId = property.id;
      td.textContent = formatValue(row.values[property.id]);
    }
    for (const id of FORMULA_IDS) {
      const td = tr.insertCell();
      td.className = "formula-cell";
      td.setAttribute("data-testid", "formula-cell");
      td.setAttribute("data-formula-id", id);
      td.textContent = "<pending>";
      formulaCells.get(id)!.push(td);
    }
  }

  return {
    root,
    mount(parent: HTMLElement) {
      parent.appendChild(root);
    },
    update(evaluation: EvaluateResult | null, error: string | null = null) {
      errorMessage.textContent = error;
      errorMessage.classList.toggle("hidden", !error);
      for (const id of FORMULA_IDS) {
        const result = evaluation?.formulas.get(id);
        const header = formulaHeaders.get(id)!;
        const outputType = result && "Ok" in result ? formatValueType(result.Ok.output_type) : null;
        header.textContent = FORMULA_DEMOS[id].label;
        header.title = outputType ?? "";
        if (outputType !== null) {
          const type = document.createElement("span");
          type.className = "table-output-type";
          type.dataset.testid = "formula-output-type";
          type.textContent = ` (${outputType})`;
          header.appendChild(type);
          header.dataset.outputType = outputType;
        } else {
          delete header.dataset.outputType;
        }
        for (const [rowIndex, cell] of formulaCells.get(id)!.entries()) {
          cell.classList.remove("is-error");
          cell.title = "";
          delete cell.dataset.errorOrigins;
          if (evaluation === null) {
            cell.textContent = error ? "Unavailable" : "<pending>";
            cell.title = error ?? "";
            continue;
          }
          if (!result || "Err" in result) {
            cell.textContent = "Not ready";
            cell.title = "Saved formula is not ready for evaluation";
            continue;
          }
          const errors = result.Ok.errors.filter((entry) => entry.row_index === rowIndex);
          if (errors.length > 0) {
            cell.textContent = "Error";
            cell.classList.add("is-error");
            cell.title = errors
              .map((entry) => `${entry.origin_formula_id}: ${formatRuntimeError(entry.error)}`)
              .join("\n");
            cell.dataset.errorOrigins = errors.map((entry) => entry.origin_formula_id).join(", ");
          } else {
            cell.textContent = formatValue(columnValue(result.Ok.column, rowIndex));
          }
        }
      }
    },
  };
}
