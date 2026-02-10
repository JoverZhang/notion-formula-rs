import { expect, test } from "@playwright/test";
import { editorContentLocator, expectEditorText, gotoDebug, setEditorContent } from "./helpers";
type EditorId = "f1" | "f2";
type Page = Parameters<typeof editorContentLocator>[0];

async function clearEditor(page: Page, id: EditorId) {
  const editor = editorContentLocator(page, id);
  await editor.click({ timeout: 10_000 });

  // Clear using user-like actions to avoid programmatic history pollution.
  await page.keyboard.press("Control+A");
  await page.keyboard.press("Backspace");
  // If running on mac in local, you can attempt Meta+A as fallback.
  await page.keyboard.press("Meta+A");
  await page.keyboard.press("Backspace");
}

async function typeInEditor(page: Page, id: EditorId, text: string) {
  const editor = editorContentLocator(page, id);
  await editor.click({ timeout: 10_000 });
  await page.keyboard.type(text);
}

async function readEditorText(page: Page, id: EditorId) {
  return page.evaluate((formulaId) => {
    const el = document.querySelector<HTMLElement>(
      `[data-testid="formula-editor"][data-formula-id="${formulaId}"] .cm-content`,
    );
    return el?.textContent ?? "";
  }, id);
}

async function waitForEditorTextChange(page: Page, id: EditorId, before: string, timeoutMs = 250) {
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

async function pressRedo(page: Page, id: EditorId, expected?: string): Promise<string> {
  const editor = editorContentLocator(page, id);
  const before = await readEditorText(page, id);

  // CM6 historyKeymap typically supports Mod-Shift-z (redo) and Mod-y.
  // We'll try a few variations for Linux/macOS.
  const attempts = ["Meta+Shift+Z", "Control+Shift+Z", "Control+Y", "Meta+Y"] as const;

  for (const combo of attempts) {
    // Ensure focus is on the correct editor instance (CI/headless can lose focus).
    await editor.click({ timeout: 10_000 });

    try {
      await page.keyboard.press(combo);

      const after = await waitForEditorTextChange(page, id, before, 3_000);

      if (expected !== undefined) {
        if (after === expected) return after;
        continue;
      }

      // No expected target provided: any change is considered success.
      if (after !== before) return after;
    } catch {
      // Shortcut didn't work or timed out waiting for change — try next combo.
      continue;
    }
  }

  // Final read (either unchanged, or changed in a way that didn't match expected).
  return await readEditorText(page, id);
}

async function pressUndo(page: Page, id: EditorId, expected?: string): Promise<string> {
  const editor = editorContentLocator(page, id);
  const before = await readEditorText(page, id);

  const attempts = ["Meta+Z", "Control+Z"] as const;

  for (const combo of attempts) {
    await editor.click({ timeout: 10_000 });

    try {
      await page.keyboard.press(combo);
      const after = await waitForEditorTextChange(page, id, before, 3_000);

      // If expected is provided, keep trying until the editor text equals expected.
      if (expected !== undefined) {
        if (after === expected) return after;
        continue;
      }

      if (after !== before) return after;
    } catch {
      continue;
    }
  }

  return await readEditorText(page, id);
}

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
});

test("undo reverts recent editor input", async ({ page }) => {
  await clearEditor(page, "f1");

  await expectEditorText(page, "f1", "");
  // CodeMirror history groups adjacent edits that happen within `newGroupDelay` (default 500ms).
  // Without a pause here, the "clear" and subsequent "type" can become a single undo group,
  // causing undo to restore the pre-clear content instead of the expected empty state.
  await page.waitForTimeout(600);

  await typeInEditor(page, "f1", "1+2");
  await expectEditorText(page, "f1", "1+2");

  await pressUndo(page, "f1");
  await expectEditorText(page, "f1", "");

  await pressRedo(page, "f1");
  await expectEditorText(page, "f1", "1+2");
});

test("editor height grows with content", async ({ page }) => {
  const HEIGHT_EPSILON_PX = 1;
  await setEditorContent(page, "f1", "a\nb\nc\nd\ne\nf\ng\nh");

  await page.waitForFunction(() => {
    const scroller = document.querySelector<HTMLElement>(
      `[data-testid="formula-editor"][data-formula-id="f1"] .cm-scroller`,
    );
    if (!scroller) return false;
    // With auto-growth, the scroller should not need internal vertical scrolling.
    return scroller.scrollHeight <= scroller.clientHeight + 1;
  });

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
    };
  });

  expect(metrics).not.toBeNull();
  if (!metrics) return;

  const lineHeightPx = Number.parseFloat(metrics.lineHeight);
  expect(Number.isFinite(lineHeightPx)).toBe(true);
  expect(lineHeightPx).toBeGreaterThan(0);

  // Loose bound: should visually grow to fit ~8 lines.
  expect(metrics.clientHeight).toBeGreaterThanOrEqual(lineHeightPx * 8 - HEIGHT_EPSILON_PX);
  expect(metrics.scrollHeight).toBeLessThanOrEqual(metrics.clientHeight + 1);
});
