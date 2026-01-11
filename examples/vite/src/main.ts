import "./style.css";
import { CONTEXT_JSON } from "./app/context";
import type { FormulaId } from "./app/types";
import { createBaseTablesView } from "./ui/base_tables_view";
import { createFormulaPanelView } from "./ui/formula_panel_view";
import { createRootLayoutView } from "./ui/layout";
import { AppVM } from "./vm/app_vm";

const FORMULA_SAMPLES: Record<FormulaId, string> = {
  f1: `if(prop("Number") > 10, prop("Text"), "Needs review")`,
  f2: `formatDate(prop("Date"), "YYYY-MM-DD")`,
  f3: `prop("Select") + " • " + prop("Text")`,
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

  const vm = new AppVM({
    contextJson: CONTEXT_JSON,
    onStateChange: (state) => {
      for (const id of Object.keys(panelViews)) {
        const view = panelViews[id];
        if (view) {
          view.update(state.formulas[id]);
        }
      }
    },
  });

  const layout = createRootLayoutView();
  layout.mount(appEl);

  const tablesView = createBaseTablesView();
  tablesView.mount(layout.slots.tables);

  for (const id of Object.keys(FORMULA_SAMPLES)) {
    const view = createFormulaPanelView({
      id,
      label: `Formula ${id}`,
      initialSource: FORMULA_SAMPLES[id],
      onSourceChange: (formulaId, source) => vm.setSource(formulaId, source),
    });
    panelViews[id] = view;
    view.mount(layout.slots.panels);
  }

  await vm.start();
  for (const id of Object.keys(FORMULA_SAMPLES)) {
    vm.setSource(id, FORMULA_SAMPLES[id]);
  }
}

await start();
