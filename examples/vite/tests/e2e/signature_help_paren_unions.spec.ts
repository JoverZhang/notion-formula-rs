import { expect, test } from "@playwright/test";
import { gotoDebug, setEditorContent, waitForCompletionDebounce } from "./helpers";

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
});

test("signature help renders parenthesized unions inside label", async ({ page }) => {
  const src = 'if(true, 42, [42, "42"])';
  await setEditorContent(page, "f1", src);

  const cursor = await page.evaluate((source) => {
    const pos = source.lastIndexOf('"42"');
    if (pos === -1) return null;
    return pos + '"42"'.length;
  }, src);
  expect(cursor).not.toBeNull();

  await page.evaluate((pos) => {
    window.__nf_debug?.setSelectionHead("f1", pos ?? 0);
  }, cursor);
  await waitForCompletionDebounce(page);

  const signature = page.locator(
    '[data-testid="completion-panel"][data-formula-id="f1"] .completion-signature',
  );
  await expect(signature).toBeVisible({ timeout: 5_000 });
  await expect(signature).toContainText("(number | string)[]");
  await expect(signature).not.toContainText("undefined");
});
