import "./style.css";
import { CONTEXT_JSON } from "./app/context";
import { FORMULA_IDS, type FormulaId } from "./app/types";
import { createFormulaPanelView } from "./ui/formula_panel_view";
import { createRootLayoutView } from "./ui/layout";
import { createFormulaTableView } from "./ui/table_view";
import { initThemeToggle } from "./ui/theme";
import { AppVM } from "./vm/app_vm";

const FORMULA_DEMOS: Record<FormulaId, { label: string; sample: string }> = {
  f1: {
    label: "Formula 1",
    sample: `if(
  sum(prop("Number"), [1, 2, 3]) > 20,
  prop("Text"),
  [
    prop("Date"),
    12,
    ["34", 56]
  ]
)`,
  },
  f2: {
    label: "Formula 2",
    sample: `(prop("Number") < 1).ifs(
  prop("Title"),
  prop("Number") < 2,
  [prop("Number")],
  prop("Number") < 3,
  prop("Date"),
  4
)`,
  },
};

const isError = (diagnostics: { kind?: string; severity?: string }[]) =>
  diagnostics.some((diag) => (diag.kind ?? diag.severity ?? "").toLowerCase() === "error");

async function start() {
  const appEl = document.querySelector<HTMLElement>("#app");
  if (!appEl) throw new Error("Missing element: #app");

  const layout = createRootLayoutView();
  layout.mount(appEl);
  initThemeToggle(layout.themeToggle);

  const tableView = createFormulaTableView();
  tableView.mount(layout.slots.tables);

  const panelViews: Partial<Record<FormulaId, ReturnType<typeof createFormulaPanelView>>> = {};
  const vm = new AppVM({
    contextJson: CONTEXT_JSON,
    onStateChange: (state) => {
      for (const id of FORMULA_IDS) panelViews[id]?.update(state.formulas[id]);
      tableView.updateFormulaStatus({
        f1: isError(state.formulas.f1.diagnostics),
        f2: isError(state.formulas.f2.diagnostics),
      });
    },
  });

  for (const id of FORMULA_IDS) {
    const meta = FORMULA_DEMOS[id];
    const view = createFormulaPanelView({
      id,
      label: meta.label,
      initialSource: meta.sample,
      onSourceChange: (formulaId, source) => vm.setSource(formulaId, source),
    });
    panelViews[id] = view;
    view.mount(layout.slots.panels);
  }

  await vm.start();
  for (const id of FORMULA_IDS) vm.setSource(id, FORMULA_DEMOS[id].sample);
}

start().catch((e) => console.error(e));
