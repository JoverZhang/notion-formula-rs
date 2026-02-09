export function createRootLayoutView() {
  const root = document.createElement("div");
  root.className = "page";
  root.setAttribute("data-testid", "app-root");
  root.innerHTML = `
    <header class="page-header">
      <div class="page-header-text">
        <h1 class="page-title">Notion Formula Demo</h1>
        <p class="page-subtitle">A lightweight playground for tokens, diagnostics, and formatting.</p>
      </div>
      <div class="page-header-actions">
        <button class="theme-toggle" type="button" data-testid="theme-toggle"></button>
      </div>
    </header>
    <section class="table-section" data-testid="table-section"></section>
    <section class="formula-section" data-testid="formula-section"></section>
  `;

  const themeToggle = root.querySelector(".theme-toggle") as HTMLButtonElement;
  const tables = root.querySelector('[data-testid="table-section"]') as HTMLElement;
  const panels = root.querySelector('[data-testid="formula-section"]') as HTMLElement;

  return {
    root,
    themeToggle,
    slots: { tables, panels },
    mount(parent: HTMLElement) {
      parent.appendChild(root);
    },
  };
}
