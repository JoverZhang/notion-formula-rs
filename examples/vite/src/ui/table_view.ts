import { FORMULA_IDS, type FormulaId } from "../app/types";

type Row = {
  id: string;
  text: string;
  number: number;
  select: "A" | "B" | "C";
  date: string;
  relation: string;
};

const ROWS: Row[] = [
  {
    id: "row-1",
    text: "Morning draft",
    number: 12,
    select: "A",
    date: "2024-05-14",
    relation: "North Star, Blueprint",
  },
  {
    id: "row-2",
    text: "Client check-in",
    number: 7,
    select: "B",
    date: "2024-06-02",
    relation: "Pulse",
  },
  {
    id: "row-3",
    text: "QA pass",
    number: 4,
    select: "C",
    date: "2024-06-09",
    relation: "Blueprint",
  },
  {
    id: "row-4",
    text: "Wrap report",
    number: 18,
    select: "A",
    date: "2024-06-16",
    relation: "North Star, Pulse",
  },
];

const COLUMNS = ["Text", "Number", "Select", "Date", "Relation", "Formula 1", "Formula 2"];

export function createFormulaTableView() {
  const root = document.createElement("div");
  root.className = "table-card";
  root.innerHTML = `
    <div>
      <h2 class="table-title">Tasks</h2>
      <p class="table-subtitle">Sample rows with formula outputs reserved on the right.</p>
    </div>
    <div class="table-scroll"></div>
  `;

  const scroll = root.querySelector(".table-scroll") as HTMLDivElement;
  const table = document.createElement("table");
  table.className = "notion-table";
  table.setAttribute("data-testid", "formula-table");
  scroll.appendChild(table);

  const head = document.createElement("thead");
  head.innerHTML = `<tr>${COLUMNS.map((label) => `<th>${label}</th>`).join("")}</tr>`;
  table.appendChild(head);

  const body = document.createElement("tbody");
  table.appendChild(body);

  const formulaCells = new Map<FormulaId, HTMLTableCellElement[]>(
    FORMULA_IDS.map((id) => [id, []]),
  );

  for (const row of ROWS) {
    const tr = document.createElement("tr");
    tr.setAttribute("data-row-id", row.id);

    const values = [row.text, String(row.number), row.select, row.date, row.relation];
    for (const value of values) {
      const td = document.createElement("td");
      td.textContent = value;
      tr.appendChild(td);
    }

    for (const id of FORMULA_IDS) {
      const td = document.createElement("td");
      td.className = "formula-cell";
      td.setAttribute("data-testid", "formula-cell");
      td.setAttribute("data-formula-id", id);
      td.textContent = "<pending>";
      formulaCells.get(id)?.push(td);
      tr.appendChild(td);
    }

    body.appendChild(tr);
  }

  return {
    root,
    mount(parent: HTMLElement) {
      parent.appendChild(root);
    },
    updateFormulaStatus(status: Partial<Record<FormulaId, boolean>>) {
      for (const id of FORMULA_IDS) {
        const text = status[id] ? "<error>" : "<pending>";
        for (const cell of formulaCells.get(id) ?? []) {
          cell.textContent = text;
          cell.classList.toggle("is-error", status[id] === true);
        }
      }
    },
  };
}
