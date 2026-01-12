import {
  Facet,
  RangeSet,
  RangeSetBuilder,
  RangeValue,
  StateEffect,
  StateField,
} from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import type { FormulaId } from "../app/types";

export type ChipDecorationRange = {
  from: number;
  to: number;
  propName: string;
  hasError?: boolean;
  hasWarning?: boolean;
  message?: string;
};

export const formulaIdFacet = Facet.define<FormulaId, FormulaId>({
  combine: (values) => values[0] ?? "f1",
});

export const setChipDecoListEffect = StateEffect.define<ChipDecorationRange[]>();

class PropChipWidget extends WidgetType {
  private readonly formulaId: FormulaId;
  private readonly propName: string;
  private readonly spanFrom: number;
  private readonly hasError: boolean;
  private readonly hasWarning: boolean;
  private readonly message: string | undefined;

  constructor(opts: {
    formulaId: FormulaId;
    propName: string;
    spanFrom: number;
    hasError?: boolean;
    hasWarning?: boolean;
    message?: string;
  }) {
    super();
    this.formulaId = opts.formulaId;
    this.propName = opts.propName;
    this.spanFrom = opts.spanFrom;
    this.hasError = opts.hasError ?? false;
    this.hasWarning = opts.hasWarning ?? false;
    this.message = opts.message;
  }

  eq(other: PropChipWidget): boolean {
    return (
      this.formulaId === other.formulaId &&
      this.propName === other.propName &&
      this.spanFrom === other.spanFrom &&
      this.hasError === other.hasError &&
      this.hasWarning === other.hasWarning &&
      this.message === other.message
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const span = document.createElement("span");
    const classes = ["nf-chip"];
    if (this.hasError) classes.push("nf-chip--error");
    if (this.hasWarning) classes.push("nf-chip--warning");
    span.className = classes.join(" ");
    span.setAttribute("data-testid", "prop-chip");
    span.setAttribute("data-formula-id", this.formulaId);
    span.setAttribute("data-prop-name", this.propName);
    span.textContent = this.propName;
    if (this.message) {
      span.title = this.message;
    }

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

class ChipRangeValue extends RangeValue {}

const chipRangeValue = new ChipRangeValue();
const emptyChipRanges = RangeSet.empty as RangeSet<ChipRangeValue>;

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
      hasError: range.hasError,
      hasWarning: range.hasWarning,
      message: range.message,
    });
    builder.add(range.from, range.to, Decoration.replace({ widget, inclusive: false }));
  }
  return builder.finish();
}

function buildChipRangeSet(ranges: ChipDecorationRange[]): RangeSet<ChipRangeValue> {
  if (!ranges || ranges.length === 0) {
    return emptyChipRanges;
  }
  const builder = new RangeSetBuilder<ChipRangeValue>();
  const sortedRanges = [...ranges].sort((a, b) => a.from - b.from || a.to - b.to);
  for (const range of sortedRanges) {
    if (range.from >= range.to) continue;
    builder.add(range.from, range.to, chipRangeValue);
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
      if (effect.is(setChipDecoListEffect)) {
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

export const chipRangesField = StateField.define<RangeSet<ChipRangeValue>>({
  create() {
    return emptyChipRanges;
  },
  update(value, tr) {
    let ranges = value;
    for (const effect of tr.effects) {
      if (effect.is(setChipDecoListEffect)) {
        ranges = buildChipRangeSet(effect.value);
      }
    }

    if (tr.docChanged) {
      ranges = ranges.map(tr.changes);
    }

    return ranges;
  },
});

export const chipAtomicRangesExt = EditorView.atomicRanges.of((view) =>
  view.state.field(chipRangesField),
);
