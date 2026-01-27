import { expect, test } from "@playwright/test";
import type { FormulaId } from "../../src/app/types";
import {
  completionItemsLocator,
  editorContentLocator,
  gotoDebug,
  setEditorContent,
  waitForAnyCompletionItems,
} from "./helpers";

const FORMULA_ID: FormulaId = "f1";
const THEME_STORAGE_KEY = "nf-theme";

function parseRgb(value: string): [number, number, number] | null {
  const m = value.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
  if (!m) return null;
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

function relativeLuminance([r, g, b]: [number, number, number]): number {
  const toLinear = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  const rl = toLinear(r);
  const gl = toLinear(g);
  const bl = toLinear(b);
  return 0.2126 * rl + 0.7152 * gl + 0.0722 * bl;
}

function contrastRatio(a: [number, number, number], b: [number, number, number]): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const lighter = Math.max(la, lb);
  const darker = Math.min(la, lb);
  return (lighter + 0.05) / (darker + 0.05);
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    localStorage.setItem(key, "dark");
    document.documentElement.dataset.theme = "dark";
  }, THEME_STORAGE_KEY);

  await gotoDebug(page);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("dark mode: CodeMirror caret has sufficient contrast", async ({ page }) => {
  const editorContent = editorContentLocator(page, FORMULA_ID);
  await editorContent.click();

  const cmEditor = page.locator(
    `[data-testid="formula-editor"][data-formula-id="${FORMULA_ID}"] .cm-editor`,
  );
  await expect(cmEditor).toHaveClass(/cm-focused/, { timeout: 5_000 });

  const colors = await page.evaluate(({ formulaId }) => {
    const scope = document.querySelector<HTMLElement>(
      `[data-testid="formula-editor"][data-formula-id="${formulaId}"]`,
    );
    if (!scope) return null;
    const content = scope.querySelector<HTMLElement>(".cm-content");
    const editor = scope.closest<HTMLElement>(".editor") ?? scope.querySelector<HTMLElement>(".editor");
    const editorEl = editor ?? scope;
    if (!content || !editorEl) return null;
    return {
      caretColor: getComputedStyle(content).caretColor,
      editorBg: getComputedStyle(editorEl).backgroundColor,
    };
  }, { formulaId: FORMULA_ID });

  expect(colors).not.toBeNull();
  if (!colors) return;

  const caretRgb = parseRgb(colors.caretColor);
  const bgRgb = parseRgb(colors.editorBg);
  expect(caretRgb, `unexpected caretColor: ${colors.caretColor}`).not.toBeNull();
  expect(bgRgb, `unexpected editorBg: ${colors.editorBg}`).not.toBeNull();
  if (!caretRgb || !bgRgb) return;

  expect(contrastRatio(caretRgb, bgRgb)).toBeGreaterThan(4.5);
});

test("dark mode: lint tooltip is readable", async ({ page }) => {
  await setEditorContent(page, FORMULA_ID, "if(");

  await page.waitForFunction(
    (formulaId) => {
      const dbg = window.__nf_debug;
      if (!dbg) return false;
      const diags = dbg.getAnalyzerDiagnostics(formulaId) ?? [];
      return diags.length > 0;
    },
    FORMULA_ID,
    { timeout: 5_000 },
  );

  const lintRange = page.locator(
    `[data-testid="formula-editor"][data-formula-id="${FORMULA_ID}"] .cm-lintRange`,
  );
  await expect(lintRange.first()).toBeVisible({ timeout: 5_000 });
  await lintRange.first().hover();

  const tooltip = page.locator(".cm-tooltip").filter({ has: page.locator(".cm-tooltip-lint") });
  await expect(tooltip.first()).toBeVisible({ timeout: 5_000 });

  const tooltipColors = await tooltip.first().evaluate((el) => {
    const bg = getComputedStyle(el).backgroundColor;
    const border = getComputedStyle(el).borderTopColor;
    const diag = el.querySelector<HTMLElement>(".cm-diagnostic");
    const diagStyle = diag ? getComputedStyle(diag) : null;
    return {
      bg,
      border,
      text: diagStyle?.color ?? getComputedStyle(el).color,
    };
  });

  const bgRgb = parseRgb(tooltipColors.bg);
  const textRgb = parseRgb(tooltipColors.text);
  const borderRgb = parseRgb(tooltipColors.border);
  expect(bgRgb, `unexpected tooltip bg: ${tooltipColors.bg}`).not.toBeNull();
  expect(textRgb, `unexpected tooltip text: ${tooltipColors.text}`).not.toBeNull();
  expect(borderRgb, `unexpected tooltip border: ${tooltipColors.border}`).not.toBeNull();
  if (!bgRgb || !textRgb || !borderRgb) return;

  expect(contrastRatio(textRgb, bgRgb)).toBeGreaterThan(4.5);
});

test("completion list has max height and internal scrolling", async ({ page }) => {
  await setEditorContent(page, FORMULA_ID, "");
  await waitForAnyCompletionItems(page, FORMULA_ID);

  const list = completionItemsLocator(page, FORMULA_ID);
  await expect
    .poll(async () => list.locator(".completion-item").count(), { timeout: 5_000 })
    .toBeGreaterThan(25);

  const listStyle = await list.evaluate((el) => {
    const cs = getComputedStyle(el);
    return { maxHeight: cs.maxHeight, overflowY: cs.overflowY };
  });

  expect(listStyle.overflowY).toBe("auto");
  expect(listStyle.maxHeight).toMatch(/px$/);
  expect(Number.parseFloat(listStyle.maxHeight)).toBeGreaterThan(0);

  await expect
    .poll(async () => list.evaluate((el) => el.scrollHeight > el.clientHeight), { timeout: 5_000 })
    .toBe(true);
});
