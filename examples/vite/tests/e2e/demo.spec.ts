import { platform } from "process";
import { expect, test, type Page } from "@playwright/test";
import type { FormulaId } from "../../src/app/types";
import "../../src/debug/common";

const SELECT_ALL = platform === "darwin" ? "Meta+A" : "Control+A";

type ChipInfo = {
  chipPos: number;
  chipStart: number;
  roundTrip: number;
  docLen: number;
  spanCount: number;
};

async function gotoDebug(page: Page) {
  await page.goto("/?debug=1");
  await page.waitForFunction(() => Boolean(window.__nf_debug), null, { timeout: 10_000 });
}

async function setEditorContent(page: Page, id: FormulaId, content: string) {
  const editor = page.locator(
    `[data-testid="formula-editor"][data-formula-id="${id}"] .cm-content`,
  );
  await editor.click();
  await page.keyboard.press(SELECT_ALL);
  await page.keyboard.type(content);
}

async function waitForTokenCount(page: Page, id: FormulaId, minCount: number) {
  await page.waitForFunction<boolean, [FormulaId, number]>(
    ([formulaId, min]) => {
      const dbg = window.__nf_debug;
      if (!dbg) return false;
      const state = dbg.getState(formulaId);
      return Boolean(state && state.tokenCount > min);
    },
    [id, minCount],
    { timeout: 5_000 },
  );
}

async function waitForDiagnostics(page: Page, id: FormulaId) {
  await page.waitForFunction(
    (formulaId) => {
      const dbg = window.__nf_debug;
      if (!dbg) return false;
      const diags = dbg.getAnalyzerDiagnostics(formulaId) ?? [];
      return diags.length > 0;
    },
    id,
    { timeout: 5_000 },
  );
}

async function waitForChipSpans(page: Page, id: FormulaId) {
  await page.waitForFunction(
    (formulaId) => {
      const dbg = window.__nf_debug;
      if (!dbg) return false;
      const spans = dbg.getChipSpans(formulaId) ?? [];
      return spans.length > 0;
    },
    id,
    { timeout: 5_000 },
  );
}

async function waitForChipUiCount(page: Page, id: FormulaId, minCount: number) {
  await page.waitForFunction(
    ({ formulaId, min }) => {
      const dbg = window.__nf_debug;
      if (!dbg) return false;
      return dbg.getChipUiCount(formulaId) >= min;
    },
    { formulaId: id, min: minCount },
    { timeout: 5_000 },
  );
}

async function waitForFormulaColumnText(page: Page, id: FormulaId, expected: string) {
  await page.waitForFunction(
    ({ formulaId, value }) => {
      const cells = document.querySelectorAll<HTMLElement>(
        `[data-testid="formula-cell"][data-formula-id="${formulaId}"]`,
      );
      if (!cells.length) return false;
      return Array.from(cells).every((cell) => cell.textContent?.trim() === value);
    },
    { formulaId: id, value: expected },
    { timeout: 5_000 },
  );
}

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
});

test("debug bridge is available and panels are registered", async ({ page }) => {
  const panels = await page.evaluate<FormulaId[]>(() => {
    const dbg = window.__nf_debug;
    return (dbg?.listPanels() ?? []).slice();
  });
  expect(panels.sort()).toEqual(["f1", "f2"]);
});

test("page layout shows table above two formula editors", async ({ page }) => {
  const table = page.locator('[data-testid="table-section"]');
  const editors = page.locator('[data-testid="formula-panel"]');
  await expect(table).toBeVisible();
  await expect(editors).toHaveCount(2);

  const tableBox = await table.boundingBox();
  const panelBox = await editors.first().boundingBox();
  expect(tableBox).not.toBeNull();
  expect(panelBox).not.toBeNull();
  if (!tableBox || !panelBox) return;
  expect(tableBox.y).toBeLessThan(panelBox.y);

  await expect(page.locator(".formula-label", { hasText: "Formula 1" })).toBeVisible();
  await expect(page.locator(".formula-label", { hasText: "Formula 2" })).toBeVisible();
});

test("formula table column shows error placeholder when parse error exists", async ({ page }) => {
  await setEditorContent(page, "f1", "if(");
  await waitForDiagnostics(page, "f1");
  await waitForFormulaColumnText(page, "f1", "<error>");

  await setEditorContent(page, "f1", 'if(prop("Number") > 1, "ok", "no")');
  await waitForFormulaColumnText(page, "f1", "<pending>");
});

test("token highlighting regression check", async ({ page }) => {
  const sample =
    'if(prop("Number") + sum(1, 2) > 3, prop("Text"), formatDate(prop("Date"), "YYYY"))';
  await setEditorContent(page, "f1", sample);
  await waitForTokenCount(page, "f1", 5);

  const tokenDecoCount = await page.evaluate<number>(() => {
    const dbg = window.__nf_debug;
    return dbg ? dbg.getTokenDecorations("f1").length : 0;
  });

  expect(tokenDecoCount).toBeGreaterThan(5);
  expect(tokenDecoCount).not.toBe(1);
});

test("diagnostics propagate to UI and CodeMirror lint", async ({ page }) => {
  await setEditorContent(page, "f1", "if(");
  await waitForDiagnostics(page, "f1");

  const cmDiagCount = await page.evaluate<number>(() => {
    const dbg = window.__nf_debug;
    return dbg ? dbg.getCmDiagnostics("f1").length : 0;
  });
  expect(cmDiagCount).toBeGreaterThan(0);

  const domDiagItems = page.locator('[data-testid="formula-diagnostics"][data-formula-id="f1"] li');
  await expect(domDiagItems.first()).toBeVisible();
  await expect(domDiagItems.first()).not.toHaveText(/No diagnostics/i);
});

test("chips remain rendered when later unterminated string causes syntax error", async ({
  page,
}) => {
  const broken = 'if(prop("Number") > 10, prop("Text"), "Needs review)';
  await setEditorContent(page, "f1", broken);
  await waitForDiagnostics(page, "f1");
  await waitForTokenCount(page, "f1", 0);

  const tokenCount = await page.evaluate<number>(() => {
    const dbg = window.__nf_debug;
    return dbg ? dbg.getState("f1").tokenCount : 0;
  });
  expect(tokenCount).toBeGreaterThan(0);

  await page.waitForFunction(() => {
    const dbg = window.__nf_debug;
    return dbg ? dbg.getChipUiCount("f1") >= 2 : false;
  });

  const decoCount = await page.evaluate<number>(() => {
    const dbg = window.__nf_debug;
    return dbg ? dbg.getTokenDecorations("f1").length : 0;
  });
  expect(decoCount).toBeGreaterThan(1);
});

test("diagnostics list uses chip positions and omits raw offsets", async ({ page }) => {
  const broken = 'if(prop("Title") == "x", "ok", "Needs review)';
  await setEditorContent(page, "f1", broken);
  await waitForDiagnostics(page, "f1");
  await waitForChipUiCount(page, "f1", 1);

  const diagText = await page
    .locator('[data-testid="formula-diagnostics"][data-formula-id="f1"]')
    .innerText();

  expect(diagText).toContain("chipPos=");
  expect(diagText).not.toMatch(/ at \d+/);
  expect(diagText).not.toMatch(/pos \d+/);
});

test("chip spans and mapping are exposed (UI not required)", async ({ page }) => {
  await setEditorContent(page, "f1", 'prop("Title")');
  await waitForTokenCount(page, "f1", 0);
  await waitForChipSpans(page, "f1");

  const chipInfo = await page.evaluate<ChipInfo | null>(() => {
    const dbg = window.__nf_debug;
    const spans = dbg?.getChipSpans("f1") ?? [];
    if (spans.length === 0 || !dbg) return null;
    const span = spans[0];
    const inside = span.start + 1;
    const chipPos = dbg.toChipPos("f1", inside);
    const chipStart = dbg.toChipPos("f1", span.start);
    const roundTrip = dbg.toRawPos("f1", chipPos);
    const docLen = dbg.getState("f1").source.length;
    return { chipPos, chipStart, roundTrip, docLen, spanCount: spans.length };
  });

  expect(chipInfo).not.toBeNull();
  expect(chipInfo?.spanCount ?? 0).toBeGreaterThanOrEqual(1);
  expect(chipInfo?.chipPos).toBe(chipInfo?.chipStart);
  expect(chipInfo?.roundTrip ?? 0).toBeGreaterThanOrEqual(0);
  expect(chipInfo?.roundTrip ?? 0).toBeLessThanOrEqual(chipInfo?.docLen ?? 0);
});

test("chip UI is rendered for valid prop(...) (enable when chip UI is implemented)", async ({
  page,
}) => {
  const sample = 'if(prop("Number") > 10, prop("Text"), "Needs review")';
  await setEditorContent(page, "f1", sample);
  await waitForTokenCount(page, "f1", 5);
  await waitForChipSpans(page, "f1");
  await waitForChipUiCount(page, "f1", 2);

  await expect(
    page.locator('[data-testid="prop-chip"][data-formula-id="f1"][data-prop-name="Number"]'),
  ).toHaveCount(1);
  await expect(
    page.locator('[data-testid="prop-chip"][data-formula-id="f1"][data-prop-name="Text"]'),
  ).toHaveCount(1);

  const tokenDecoCount = await page.evaluate<number>(() => {
    const dbg = window.__nf_debug;
    return dbg ? dbg.getTokenDecorations("f1").length : 0;
  });

  expect(tokenDecoCount).toBeGreaterThan(5);
  expect(tokenDecoCount).not.toBe(1);
});

test("arrow navigation jumps over chip ranges", async ({ page }) => {
  await setEditorContent(page, "f1", 'prop("Title")');
  await waitForTokenCount(page, "f1", 0);
  await waitForChipUiCount(page, "f1", 1);

  const chipRange = await page.evaluate(() => {
    const dbg = window.__nf_debug;
    if (!dbg) return null;
    const ranges = dbg.getChipUiRanges("f1");
    return ranges.length ? { from: ranges[0].from, to: ranges[0].to } : null;
  });

  expect(chipRange).not.toBeNull();
  if (!chipRange) return;

  await page.evaluate((pos) => {
    const dbg = window.__nf_debug;
    dbg?.setSelectionHead("f1", pos);
  }, chipRange.to);
  await page.keyboard.press("ArrowLeft");

  const leftHead = await page.evaluate(() => {
    const dbg = window.__nf_debug;
    return dbg?.getSelectionHead("f1") ?? -1;
  });
  expect(leftHead).toBe(chipRange.from);

  await page.evaluate((pos) => {
    const dbg = window.__nf_debug;
    dbg?.setSelectionHead("f1", pos);
  }, chipRange.from);
  await page.keyboard.press("ArrowRight");

  const rightHead = await page.evaluate(() => {
    const dbg = window.__nf_debug;
    return dbg?.getSelectionHead("f1") ?? -1;
  });
  expect(rightHead).toBe(chipRange.to);
});

test("chips reflect diagnostics when ranges intersect", async ({ page }) => {
  await setEditorContent(page, "f1", 'if(prop("Title"))');
  await waitForTokenCount(page, "f1", 0);
  await waitForDiagnostics(page, "f1");
  await waitForChipUiCount(page, "f1", 1);

  const chipStatus = await page.evaluate(() => {
    const dbg = window.__nf_debug;
    if (!dbg) return { count: 0, flagged: 0 };
    const ranges = dbg.getChipUiRanges("f1") ?? [];
    const flagged = ranges.filter((range) => range.hasError || range.hasWarning).length;
    return { count: ranges.length, flagged };
  });

  expect(chipStatus.count).toBeGreaterThan(0);
  expect(chipStatus.flagged).toBeGreaterThan(0);
});
