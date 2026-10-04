import "./style.css";
import { FORMULA_DEMOS } from "./app/context";
import { FORMULA_IDS, type FormulaId } from "./app/types";
import { createFormulaPanelView } from "./ui/formula_panel_view";
import { createRootLayoutView } from "./ui/layout";
import { createFormulaTableView } from "./ui/table_view";
import { initThemeToggle } from "./ui/theme";
import { AppVM } from "./vm/app_vm";

async function start() {
  const appEl = document.querySelector<HTMLElement>("#app");
  if (!appEl) throw new Error("Missing element: #app");

  const layout = createRootLayoutView();
  layout.mount(appEl);
  const disposeTheme = initThemeToggle(layout.themeToggle);

  const errorBanner = document.createElement("p");
  errorBanner.className = "app-error";
  errorBanner.setAttribute("role", "alert");
  errorBanner.setAttribute("data-testid", "app-error");
  errorBanner.hidden = true;
  layout.slots.tables.before(errorBanner);

  const tableView = createFormulaTableView();
  tableView.mount(layout.slots.tables);

  const panelViews: Partial<Record<FormulaId, ReturnType<typeof createFormulaPanelView>>> = {};
  let disposed = false;
  const vm = new AppVM({
    onStateChange: (state) => {
      if (disposed) return;
      for (const id of FORMULA_IDS) panelViews[id]?.update(state.formulas[id], state.saving);
      tableView.update(state.evaluation, state.error);
      errorBanner.textContent = state.error ?? "";
      errorBanner.hidden = !state.error;
    },
  });

  for (const id of FORMULA_IDS) {
    const meta = FORMULA_DEMOS[id];
    const view = createFormulaPanelView({
      id,
      label: meta.label,
      initialSource: meta.sample,
      actions: vm,
    });
    panelViews[id] = view;
    view.mount(layout.slots.panels);
  }

  const dispose = () => {
    if (disposed) return;
    disposed = true;
    window.removeEventListener("pagehide", onPageHide);
    disposeTheme();
    for (const id of FORMULA_IDS) panelViews[id]?.dispose();
    layout.root.remove();
    void vm.dispose().catch((error) => console.error("Unable to close formula session", error));
  };
  // A persisted page remains alive in the back-forward cache; its Worker and
  // unsaved buffers resume when the browser restores it.
  const onPageHide = (event: PageTransitionEvent) => {
    if (!event.persisted) dispose();
  };
  window.addEventListener("pagehide", onPageHide);
  import.meta.hot?.dispose(dispose);

  try {
    await vm.start();
  } catch (error) {
    if (disposed) return;
    if (!errorBanner.textContent) {
      errorBanner.textContent = error instanceof Error ? error.message : String(error);
    }
    errorBanner.hidden = false;
  }
}

void start().catch((error) => {
  console.error("Unable to start formula demo", error);
  const app = document.querySelector<HTMLElement>("#app");
  if (app) {
    app.textContent = error instanceof Error ? error.message : String(error);
    app.setAttribute("role", "alert");
  }
});
