type RelatedRow = {
  id: string;
  Text: string;
  Number: number;
  Select: "A" | "B" | "C";
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
  { id: "rel-1", Text: "North Star", Number: 3, Select: "A" },
  { id: "rel-2", Text: "Blueprint", Number: 8, Select: "B" },
  { id: "rel-3", Text: "Pulse", Number: 5, Select: "C" },
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

function createBaseTableSection(): HTMLElement {
  const section = document.createElement("section");
  section.className = "base-section pane";

  const title = document.createElement("h1");
  title.textContent = "Base Table";
  section.appendChild(title);

  const subtitle = document.createElement("p");
  subtitle.className = "section-subtitle";
  subtitle.textContent = "Read-only mock data with a relation into RelatedTable.";
  section.appendChild(subtitle);

  const grid = document.createElement("div");
  grid.className = "table-grid";

  const relatedMap = new Map(RELATED_TABLE.map((row) => [row.id, row]));

  const baseCard = document.createElement("div");
  baseCard.className = "table-card";
  const baseCardTitle = document.createElement("h2");
  baseCardTitle.textContent = "Tasks";
  baseCard.appendChild(baseCardTitle);

  const baseTable = document.createElement("table");
  baseTable.appendChild(createTableHeader(["Text", "Number", "Select", "Date", "Relation"]));
  const baseBody = document.createElement("tbody");

  BASE_ROWS.forEach((row) => {
    const tr = document.createElement("tr");

    [row.Text, row.Number.toString(), row.Select, row.Date].forEach((value) => {
      const td = document.createElement("td");
      td.textContent = value;
      tr.appendChild(td);
    });

    const relationTd = document.createElement("td");
    const pillWrap = document.createElement("div");
    pillWrap.className = "rel-pill-group";
    row.Relation.forEach((relId) => {
      const relRow = relatedMap.get(relId);
      const pill = document.createElement("span");
      pill.className = "rel-pill";
      pill.textContent = relRow ? relRow.Text : relId;
      pillWrap.appendChild(pill);
    });
    relationTd.appendChild(pillWrap);
    tr.appendChild(relationTd);

    baseBody.appendChild(tr);
  });

  baseTable.appendChild(baseBody);
  baseCard.appendChild(baseTable);

  const relatedCard = document.createElement("div");
  relatedCard.className = "table-card";
  const relatedTitle = document.createElement("h2");
  relatedTitle.textContent = "RelatedTable";
  relatedCard.appendChild(relatedTitle);

  const relatedTable = document.createElement("table");
  relatedTable.appendChild(createTableHeader(["Text", "Number", "Select"]));
  const relatedBody = document.createElement("tbody");
  RELATED_TABLE.forEach((row) => {
    const tr = document.createElement("tr");
    [row.Text, row.Number.toString(), row.Select].forEach((value) => {
      const td = document.createElement("td");
      td.textContent = value;
      tr.appendChild(td);
    });
    relatedBody.appendChild(tr);
  });
  relatedTable.appendChild(relatedBody);
  relatedCard.appendChild(relatedTable);

  grid.appendChild(baseCard);
  grid.appendChild(relatedCard);
  section.appendChild(grid);

  return section;
}

export function createBaseTablesView(): { root: HTMLElement; mount(parent: HTMLElement): void } {
  const root = createBaseTableSection();
  return {
    root,
    mount(parent: HTMLElement) {
      parent.appendChild(root);
    },
  };
}
