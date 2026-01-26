import type { FormulaId } from "../app/types";

type RelatedRow = {
  id: string;
  Text: string;
};

type BaseRow = {
  id: string;
  Text: string;
  Number: number;
  Select: "A" | "B" | "C";
  Date: string;
  Relation: string[];
};

const RELATED_TABLE: RelatedRow[] = [
  { id: "rel-1", Text: "North Star" },
  { id: "rel-2", Text: "Blueprint" },
  { id: "rel-3", Text: "Pulse" },
];

const BASE_ROWS: BaseRow[] = [
  {
    id: "row-1",
    Text: "Morning draft",
    Number: 12,
    Select: "A",
    Date: "2024-05-14",
    Relation: ["rel-1", "rel-2"],
  },
  {
    id: "row-2",
    Text: "Client check-in",
    Number: 7,
    Select: "B",
    Date: "2024-06-02",
    Relation: ["rel-3"],
  },
  {
    id: "row-3",
    Text: "QA pass",
    Number: 4,
    Select: "C",
    Date: "2024-06-09",
    Relation: ["rel-2"],
  },
  {
    id: "row-4",
    Text: "Wrap report",
    Number: 18,
    Select: "A",
    Date: "2024-06-16",
    Relation: ["rel-1", "rel-3"],
  },
];

const FORMULA_IDS: FormulaId[] = ["f1", "f2"];

const COLUMN_LABELS = [
  "Text",
  "Number",
  "Select",
  "Date",
  "Relation",
  "Formula 1",
  "Formula 2",
];

function createTableHeader(labels: string[]): HTMLTableSectionElement {
  const thead = document.createElement("thead");
  const row = document.createElement("tr");
  labels.forEach((label) => {
    const th = document.createElement("th");
    th.textContent = label;
    row.appendChild(th);
  });
  thead.appendChild(row);
  return thead;
}

function resolveRelations(ids: string[], lookup: Map<string, RelatedRow>): string[] {
  return ids.map((id) => lookup.get(id)?.Text ?? id);
}

export function createFormulaTableView() {
  const root = document.createElement("div");
  root.className = "table-card";

  const header = document.createElement("div");
  header.className = "table-card-header";

  const titleWrap = document.createElement("div");
  const title = document.createElement("h2");
  title.className = "table-title";
  title.textContent = "Tasks";
  const subtitle = document.createElement("p");
  subtitle.className = "table-subtitle";
  subtitle.textContent = "Sample rows with formula outputs reserved on the right.";
  titleWrap.append(title, subtitle);

  header.appendChild(titleWrap);
  root.appendChild(header);

  const scroll = document.createElement("div");
  scroll.className = "table-scroll";

  const table = document.createElement("table");
  table.className = "notion-table";
  table.setAttribute("data-testid", "formula-table");
  table.appendChild(createTableHeader(COLUMN_LABELS));

  const body = document.createElement("tbody");
  const relatedMap = new Map(RELATED_TABLE.map((row) => [row.id, row]));
  const formulaCells = new Map<FormulaId, HTMLTableCellElement[]>(
    FORMULA_IDS.map((id) => [id, []]),
  );

  BASE_ROWS.forEach((row) => {
    const tr = document.createElement("tr");
    tr.setAttribute("data-row-id", row.id);

    const relationNames = resolveRelations(row.Relation, relatedMap);
    const baseValues = [row.Text, row.Number.toString(), row.Select, row.Date];
    baseValues.forEach((value) => {
      const td = document.createElement("td");
      td.textContent = value;
      tr.appendChild(td);
    });

    const relationTd = document.createElement("td");
    const tags = document.createElement("div");
    tags.className = "tag-group";
    relationNames.forEach((label) => {
      const tag = document.createElement("span");
      tag.className = "tag";
      tag.textContent = label;
      tags.appendChild(tag);
    });
    relationTd.appendChild(tags);
    tr.appendChild(relationTd);

    FORMULA_IDS.forEach((id) => {
      const td = document.createElement("td");
      td.className = "formula-cell";
      td.setAttribute("data-testid", "formula-cell");
      td.setAttribute("data-formula-id", id);
      td.textContent = "<pending>";
      formulaCells.get(id)?.push(td);
      tr.appendChild(td);
    });

    body.appendChild(tr);
  });

  table.appendChild(body);
  scroll.appendChild(table);
  root.appendChild(scroll);

  function updateFormulaStatus(status: Partial<Record<FormulaId, boolean>>) {
    FORMULA_IDS.forEach((id) => {
      const cells = formulaCells.get(id) ?? [];
      const isError = Boolean(status[id]);
      const text = isError ? "<error>" : "<pending>";
      cells.forEach((cell) => {
        cell.textContent = text;
        cell.classList.toggle("is-error", isError);
      });
    });
  }

  return {
    root,
    mount(parent: HTMLElement) {
      parent.appendChild(root);
    },
    updateFormulaStatus,
  };
}
