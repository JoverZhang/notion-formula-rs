import { expect, test, type Page } from "@playwright/test";
import type { FormulaId } from "../../src/app/types";
import { gotoDebug, setEditorContent, waitForCompletionDebounce } from "./helpers";

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
  await page.setViewportSize({ width: 1600, height: 900 });
});

async function setCursorAfter(page: Page, formulaId: FormulaId, needle: string) {
  await page.evaluate(
    ({ id, needleText }) => {
      const dbg = window.__nf_debug;
      if (!dbg) throw new Error("Missing __nf_debug");
      const source = dbg.getState(id).source;
      const idx = source.indexOf(needleText);
      if (idx === -1) throw new Error(`Missing needle: ${needleText}`);
      dbg.setSelectionHead(id, idx + needleText.length);
    },
    { id: formulaId, needleText: needle },
  );
}

test("Suggestion signature follows focus and hides on editor blur", async ({ page }) => {
  await setEditorContent(page, "Formula 1", 'if(true, 1, "x")');
  await setCursorAfter(page, "Formula 1", '"x"');
  const editor = page.locator('[data-testid="formula-editor"][data-formula-id="Formula 1"]');
  const editorBoxBefore = await editor.boundingBox();
  expect(editorBoxBefore).not.toBeNull();
  if (!editorBoxBefore) return;

  await waitForCompletionDebounce(page);

  const signature = page.locator(
    '[data-testid="suggestion-signature"][data-formula-id="Formula 1"]',
  );
  const completionPanel = page.locator(
    '[data-testid="completion-panel"][data-formula-id="Formula 1"]',
  );
  await expect(signature).toBeVisible({ timeout: 5_000 });
  await expect(completionPanel).toBeVisible({ timeout: 5_000 });

  const signatureBox = await signature.boundingBox();
  const editorBoxAfter = await editor.boundingBox();
  expect(signatureBox).not.toBeNull();
  expect(editorBoxAfter).not.toBeNull();
  if (!signatureBox || !editorBoxBefore || !editorBoxAfter) return;

  expect(signatureBox.y).toBeGreaterThanOrEqual(editorBoxAfter.y + editorBoxAfter.height);
  expect(signatureBox.x).toBeGreaterThanOrEqual(editorBoxAfter.x - 1);
  expect(signatureBox.x + signatureBox.width).toBeLessThanOrEqual(
    editorBoxAfter.x + editorBoxAfter.width + 1,
  );

  const viewport = page.viewportSize();
  expect(viewport).not.toBeNull();
  if (!viewport) return;
  expect(signatureBox.x).toBeGreaterThanOrEqual(0);
  expect(signatureBox.x + signatureBox.width).toBeLessThanOrEqual(viewport.width + 1);

  // In-flow help keeps the editor's geometry stable.
  expect(Math.abs(editorBoxBefore.width - editorBoxAfter.width)).toBeLessThan(1);
  expect(Math.abs(editorBoxBefore.y - editorBoxAfter.y)).toBeLessThan(1);

  const editorWrap = page.locator(
    '[data-testid="formula-panel"][data-formula-id="Formula 1"] .formula-editor-wrap',
  );
  const editorWrapBox = await editorWrap.boundingBox();
  const completionBox = await completionPanel.boundingBox();
  expect(editorWrapBox).not.toBeNull();
  expect(completionBox).not.toBeNull();
  if (!editorWrapBox || !completionBox) return;
  expect(completionBox.y).toBeGreaterThan(editorWrapBox.y);
  expect(completionBox.y).toBeLessThan(editorWrapBox.y + editorWrapBox.height + 1);

  await page.locator('[data-testid="theme-toggle"]').click();
  await expect(signature).toBeHidden({ timeout: 5_000 });
  await expect(completionPanel).toBeHidden({ timeout: 5_000 });

  await setEditorContent(page, "Formula 2", 'if(true, 1, "x")');
  await setCursorAfter(page, "Formula 2", '"x"');
  await waitForCompletionDebounce(page);

  const signature2 = page.locator(
    '[data-testid="suggestion-signature"][data-formula-id="Formula 2"]',
  );
  const completionPanel2 = page.locator(
    '[data-testid="completion-panel"][data-formula-id="Formula 2"]',
  );
  await expect(signature2).toBeVisible({ timeout: 5_000 });
  await expect(completionPanel2).toBeVisible({ timeout: 5_000 });

  await expect(signature).toBeHidden({ timeout: 5_000 });
  await expect(completionPanel).toBeHidden({ timeout: 5_000 });
});
