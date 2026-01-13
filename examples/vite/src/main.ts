import "./style.css";
import { CONTEXT_JSON } from "./app/context";
import type { FormulaId } from "./app/types";
import { createFormulaPanelView } from "./ui/formula_panel_view";
import { createRootLayoutView } from "./ui/layout";
import { createFormulaTableView } from "./ui/table_view";
import { initThemeToggle } from "./ui/theme";
import { AppVM } from "./vm/app_vm";

const FORMULA_DEMOS: Record<FormulaId, { label: string; sample: string; note?: string }> = {
  f1: {
    label: "Formula 1",
    sample: `if(prop("Number") > 10, prop("Text"), "Needs review")`,
  },
  f2: {
    label: "Formula 2",
    sample: `formatDate(prop("Date"), "YYYY-MM-DD")`,
  },
  f3: {
    label: "Formula 3",
    sample: `prop("Select") + " • " + prop("Text")`,
    note: "May reference Formula 1 & 2.",
  },
};

function expectEl<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) {
    throw new Error(`Missing element: ${selector}`);
  }
  return el;
}

async function start() {
  const appEl = expectEl<HTMLElement>("#app");
  const panelViews: Partial<Record<FormulaId, ReturnType<typeof createFormulaPanelView>>> = {};

  const layout = createRootLayoutView();
  layout.mount(appEl);
  initThemeToggle(layout.themeToggle);

  const tableView = createFormulaTableView();
  tableView.mount(layout.slots.tables);

  const vm = new AppVM({
    contextJson: CONTEXT_JSON,
    onStateChange: (state) => {
      for (const id of Object.keys(panelViews)) {
        const view = panelViews[id];
        if (view) {
          view.update(state.formulas[id]);
        }
      }
      tableView.updateFormulaStatus({
        f1: hasError(state.formulas.f1.diagnostics),
        f2: hasError(state.formulas.f2.diagnostics),
        f3: hasError(state.formulas.f3.diagnostics),
      });
    },
  });

  for (const id of Object.keys(FORMULA_DEMOS)) {
    const meta = FORMULA_DEMOS[id];
    const view = createFormulaPanelView({
      id,
      label: meta.label,
      note: meta.note,
      initialSource: meta.sample,
      onSourceChange: (formulaId, source) => vm.setSource(formulaId, source),
    });
    panelViews[id] = view;
    view.mount(layout.slots.panels);
  }

  await vm.start();
  for (const id of Object.keys(FORMULA_DEMOS)) {
    vm.setSource(id, FORMULA_DEMOS[id].sample);
  }
}

function hasError(diagnostics: { kind?: string; severity?: string }[]): boolean {
  return diagnostics.some((diag) => {
    const raw = (diag.kind ?? diag.severity ?? "").toLowerCase();
    return raw === "error";
  });
}

start().catch((e) => console.error(e));
