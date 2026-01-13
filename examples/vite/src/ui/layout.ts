export function createRootLayoutView() {
  const root = document.createElement("div");
  root.className = "page";
  root.setAttribute("data-testid", "app-root");

  const header = document.createElement("header");
  header.className = "page-header";

  const headerText = document.createElement("div");
  headerText.className = "page-header-text";

  const title = document.createElement("h1");
  title.className = "page-title";
  title.textContent = "Notion Formula Demo";

  const subtitle = document.createElement("p");
  subtitle.className = "page-subtitle";
  subtitle.textContent = "A lightweight playground for tokens, diagnostics, and formatting.";

  headerText.append(title, subtitle);

  const headerActions = document.createElement("div");
  headerActions.className = "page-header-actions";
  const themeToggle = document.createElement("button");
  themeToggle.className = "theme-toggle";
  themeToggle.type = "button";
  themeToggle.setAttribute("data-testid", "theme-toggle");
  headerActions.appendChild(themeToggle);

  header.append(headerText, headerActions);

  const tables = document.createElement("section");
  tables.className = "table-section";
  tables.setAttribute("data-testid", "table-section");

  const panels = document.createElement("section");
  panels.className = "formula-section";
  panels.setAttribute("data-testid", "formula-section");

  root.append(header, tables, panels);

  return {
    root,
    themeToggle,
    slots: { tables, panels },
    mount(parent: HTMLElement) {
      parent.appendChild(root);
    },
  };
}
