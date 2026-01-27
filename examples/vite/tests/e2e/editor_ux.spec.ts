import { expect, test } from "@playwright/test";
import { editorContentLocator, expectEditorText, gotoDebug, setEditorContent } from "./helpers";

async function readEditorText(page: Parameters<typeof editorContentLocator>[0], id: "f1" | "f2") {
  return page.evaluate((formulaId) => {
    const el = document.querySelector<HTMLElement>(
      `[data-testid="formula-editor"][data-formula-id="${formulaId}"] .cm-content`,
    );
    return el?.textContent ?? "";
  }, id);
}

async function waitForEditorTextChange(
  page: Parameters<typeof editorContentLocator>[0],
  id: "f1" | "f2",
  before: string,
  timeoutMs = 250,
) {
  try {
    await page.waitForFunction(
      ({ formulaId, prev }) => {
        const el = document.querySelector<HTMLElement>(
          `[data-testid="formula-editor"][data-formula-id="${formulaId}"] .cm-content`,
        );
        return (el?.textContent ?? "") !== prev;
      },
      { formulaId: id, prev: before },
      { timeout: timeoutMs },
    );
  } catch {
    // Ignore timeouts; caller decides how to handle no-change.
  }
  return readEditorText(page, id);
}

async function pressUndo(page: Parameters<typeof editorContentLocator>[0], id: "f1" | "f2") {
  const before = await readEditorText(page, id);
  await page.keyboard.press("Meta+Z");
  let after = await waitForEditorTextChange(page, id, before);
  if (after === before) {
    await page.keyboard.press("Control+Z");
    after = await waitForEditorTextChange(page, id, before);
  }
  return after;
}

async function pressRedo(page: Parameters<typeof editorContentLocator>[0], id: "f1" | "f2") {
  const before = await readEditorText(page, id);
  const attempts = ["Meta+Shift+Z", "Control+Shift+Z", "Control+Y", "Meta+Y"];
  for (const combo of attempts) {
    await page.keyboard.press(combo);
    const after = await waitForEditorTextChange(page, id, before);
    if (after !== before) return after;
  }
  return readEditorText(page, id);
}

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
});

test("undo reverts recent editor input", async ({ page }) => {
  await setEditorContent(page, "f1", "");
  const editor = editorContentLocator(page, "f1");
  await editor.click();
  await page.keyboard.type("1+2");
  await expectEditorText(page, "f1", "1+2");

  const afterUndo = await pressUndo(page, "f1");
  expect(afterUndo).not.toBe("1+2");

  const afterRedo = await pressRedo(page, "f1");
  expect(afterRedo).toBe("1+2");
});

test("editor height is capped and content scrolls", async ({ page }) => {
  await setEditorContent(page, "f1", "a\nb\nc\nd\ne\nf\n");

  const metrics = await page.evaluate(() => {
    const scroller = document.querySelector<HTMLElement>(
      `[data-testid="formula-editor"][data-formula-id="f1"] .cm-scroller`,
    );
    if (!scroller) return null;
    const cs = getComputedStyle(scroller);
    return {
      clientHeight: scroller.clientHeight,
      scrollHeight: scroller.scrollHeight,
      lineHeight: cs.lineHeight,
      overflowY: cs.overflowY,
    };
  });

  expect(metrics).not.toBeNull();
  if (!metrics) return;
  expect(metrics.overflowY).toMatch(/auto|scroll/i);

  const lineHeightPx = Number.parseFloat(metrics.lineHeight);
  expect(Number.isFinite(lineHeightPx)).toBe(true);
  expect(lineHeightPx).toBeGreaterThan(0);

  // Loose bound: should not visually grow to 6+ lines.
  expect(metrics.clientHeight).toBeLessThanOrEqual(lineHeightPx * 6);
  expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);
});
