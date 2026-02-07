import { expect, test, type Page } from "@playwright/test";
import { gotoDebug, setEditorContent, waitForCompletionDebounce } from "./helpers";

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
  await page.setViewportSize({ width: 1600, height: 900 });
});

async function setCursorAfter(page: Page, formulaId: string, needle: string) {
  await page.evaluate(
    ({ id, needleText }) => {
      const dbg = window.__nf_debug;
      if (!dbg) throw new Error("Missing __nf_debug");
      const source = dbg.getState(id as any).source;
      const idx = source.indexOf(needleText);
      if (idx === -1) throw new Error(`Missing needle: ${needleText}`);
      dbg.setSelectionHead(id as any, idx + needleText.length);
    },
    { id: formulaId, needleText: needle },
  );
}

test("Suggestion signature pops left and stays until another editor is focused", async ({ page }) => {
  await setEditorContent(page, "f1", 'if(true, 1, "x")');
  await setCursorAfter(page, "f1", '"x"');
  const editor = page.locator('[data-testid="formula-editor"][data-formula-id="f1"]');
  const editorBoxBefore = await editor.boundingBox();
  expect(editorBoxBefore).not.toBeNull();
  if (!editorBoxBefore) return;

  await waitForCompletionDebounce(page);

  const signature = page.locator('[data-testid="suggestion-signature"][data-formula-id="f1"]');
  const completionPanel = page.locator('[data-testid="completion-panel"][data-formula-id="f1"]');
  await expect(signature).toBeVisible({ timeout: 5_000 });
  await expect(completionPanel).toBeVisible({ timeout: 5_000 });

  const signatureBox = await signature.boundingBox();
  const editorBoxAfter = await editor.boundingBox();
  expect(signatureBox).not.toBeNull();
  expect(editorBoxAfter).not.toBeNull();
  if (!signatureBox || !editorBoxBefore || !editorBoxAfter) return;

  const side = await signature.getAttribute("data-side");
  expect(side).toMatch(/^(left|right)$/);

  if (side === "left") {
    expect(signatureBox.x + signatureBox.width).toBeLessThanOrEqual(editorBoxAfter.x);
  } else {
    expect(signatureBox.x).toBeGreaterThanOrEqual(editorBoxAfter.x + editorBoxAfter.width);
  }

  const viewport = page.viewportSize();
  expect(viewport).not.toBeNull();
  if (!viewport) return;
  expect(signatureBox.x).toBeGreaterThanOrEqual(0);
  expect(signatureBox.x + signatureBox.width).toBeLessThanOrEqual(viewport.width + 1);

  // Popover doesn't change the editor width.
  expect(Math.abs(editorBoxBefore.width - editorBoxAfter.width)).toBeLessThan(1);

  const diagList = page.locator('[data-testid="formula-diagnostics"][data-formula-id="f1"]');
  const diagBox = await diagList.boundingBox();
  const completionBox = await completionPanel.boundingBox();
  expect(diagBox).not.toBeNull();
  expect(completionBox).not.toBeNull();
  if (!diagBox || !completionBox) return;
  expect(completionBox.y).toBeGreaterThan(diagBox.y + diagBox.height - 1);

  await page.locator('[data-testid="theme-toggle"]').click();
  await expect(signature).toBeVisible({ timeout: 5_000 });
  await expect(completionPanel).toBeVisible({ timeout: 5_000 });

  await setEditorContent(page, "f2", 'if(true, 1, "x")');
  await setCursorAfter(page, "f2", '"x"');
  await waitForCompletionDebounce(page);

  const signature2 = page.locator('[data-testid="suggestion-signature"][data-formula-id="f2"]');
  const completionPanel2 = page.locator('[data-testid="completion-panel"][data-formula-id="f2"]');
  await expect(signature2).toBeVisible({ timeout: 5_000 });
  await expect(completionPanel2).toBeVisible({ timeout: 5_000 });

  await expect.poll(() => signature.getAttribute("data-side")).toBe("right");
  await expect.poll(() => signature2.getAttribute("data-side")).toBe("left");

  await expect(signature).toBeHidden({ timeout: 5_000 });
  await expect(completionPanel).toBeHidden({ timeout: 5_000 });
});
