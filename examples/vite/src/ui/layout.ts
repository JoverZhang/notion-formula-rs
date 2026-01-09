export function createRootLayoutView() {
  const root = document.createElement("div");
  root.className = "layout";

  const tables = document.createElement("div");
  const divider = document.createElement("hr");
  divider.className = "divider";
  const panels = document.createElement("section");
  panels.className = "formula-section";

  root.append(tables, divider, panels);

  return {
    root,
    slots: { tables, panels },
    mount(parent: HTMLElement) { parent.appendChild(root); },
  };
}
