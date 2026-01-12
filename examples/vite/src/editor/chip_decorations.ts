import { Facet, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import type { FormulaId } from "../app/types";

export type ChipDecorationRange = {
  from: number;
  to: number;
  propName: string;
};

export const formulaIdFacet = Facet.define<FormulaId, FormulaId>({
  combine: (values) => values[0] ?? "f1",
});

export const setChipDecosEffect = StateEffect.define<ChipDecorationRange[]>();

class PropChipWidget extends WidgetType {
  private readonly formulaId: FormulaId;
  private readonly propName: string;
  private readonly spanFrom: number;

  constructor(opts: { formulaId: FormulaId; propName: string; spanFrom: number }) {
    super();
    this.formulaId = opts.formulaId;
    this.propName = opts.propName;
    this.spanFrom = opts.spanFrom;
  }

  eq(other: PropChipWidget): boolean {
    return (
      this.formulaId === other.formulaId &&
      this.propName === other.propName &&
      this.spanFrom === other.spanFrom
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const span = document.createElement("span");
    span.className = "nf-chip";
    span.setAttribute("data-testid", "prop-chip");
    span.setAttribute("data-formula-id", this.formulaId);
    span.setAttribute("data-prop-name", this.propName);
    span.textContent = this.propName;

    span.addEventListener("click", (event) => {
      event.preventDefault();
      view.dispatch({ selection: { anchor: this.spanFrom } });
      view.focus();
    });

    return span;
  }

  ignoreEvent(event: Event): boolean {
    return event.type === "mousedown" || event.type === "mouseup" || event.type === "click";
  }
}

function buildChipDecorationSet(
  ranges: ChipDecorationRange[],
  formulaId: FormulaId,
): DecorationSet {
  if (!ranges || ranges.length === 0) {
    return Decoration.none;
  }

  const builder = new RangeSetBuilder<Decoration>();
  const sortedRanges = [...ranges].sort((a, b) => a.from - b.from || a.to - b.to);
  for (const range of sortedRanges) {
    if (range.from >= range.to) continue;
    const widget = new PropChipWidget({
      formulaId,
      propName: range.propName,
      spanFrom: range.from,
    });
    builder.add(range.from, range.to, Decoration.replace({ widget, inclusive: false }));
  }
  return builder.finish();
}

export const chipDecoStateField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(value, tr) {
    let decorations = value;
    for (const effect of tr.effects) {
      if (effect.is(setChipDecosEffect)) {
        const formulaId = tr.state.facet(formulaIdFacet);
        decorations = buildChipDecorationSet(effect.value, formulaId);
      }
    }

    if (tr.docChanged) {
      decorations = decorations.map(tr.changes);
    }

    return decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});
