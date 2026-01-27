import { expect, test } from "@playwright/test";
import type { FormulaId } from "../../src/app/types";
import {
  applyCompletionByDomClick,
  completionItemsLocator,
  expectEditorText,
  expectCursorAfter,
  gotoDebug,
  setEditorContent,
  waitForAnyCompletionItems,
  waitForCompletionDebounce,
} from "./helpers";

const FORMULA_ID: FormulaId = "f1";

test.beforeEach(async ({ page }) => {
  await gotoDebug(page);
});

test("cursor is placed correctly after applying a completion", async ({ page }) => {
  await setEditorContent(page, FORMULA_ID, "i");
  await waitForAnyCompletionItems(page, FORMULA_ID);
  await waitForCompletionDebounce(page);

  await applyCompletionByDomClick(page, FORMULA_ID, "if");
  await expectEditorText(page, FORMULA_ID, "if()");

  await expectCursorAfter(page, FORMULA_ID, "if(");
});

test("selected completion item is scrolled into view", async ({ page }) => {
  await setEditorContent(page, FORMULA_ID, "");
  await waitForAnyCompletionItems(page, FORMULA_ID);
  await waitForCompletionDebounce(page);

  const list = completionItemsLocator(page, FORMULA_ID);
  const selected = list.locator(".completion-item.is-selected");
  await expect(selected).toHaveCount(1);

  // Force the list to be a scroll container (some layouts may expand it enough that it doesn't
  // naturally overflow, making scrollTop assertions meaningless).
  await list.evaluate((el) => {
    el.style.maxHeight = "60px";
    el.style.overflowY = "auto";
    el.scrollTop = 0;
  });
  await expect
    .poll(async () => list.evaluate((el) => el.scrollHeight > el.clientHeight), { timeout: 5_000 })
    .toBe(true);

  // Select a far item by dispatching a mouseenter event (avoids focus flake in headless runs).
  const targetLabel = "replaceAll";
  await page.evaluate(
    ({ formulaId, label }) => {
      const panel = document.querySelector(
        `[data-testid="completion-panel"][data-formula-id="${formulaId}"]`,
      );
      if (!panel) throw new Error("Missing completion panel");
      const items = panel.querySelectorAll<HTMLElement>(".completion-item");
      const target = Array.from(items).find(
        (item) => item.querySelector(".completion-item-label")?.textContent?.trim() === label,
      );
      if (!target) throw new Error(`Missing completion item: ${label}`);
      target.dispatchEvent(new MouseEvent("mouseenter", { bubbles: true }));
    },
    { formulaId: FORMULA_ID, label: targetLabel },
  );

  await expect
    .poll(async () => list.evaluate((el) => el.scrollTop), { timeout: 5_000 })
    .toBeGreaterThan(0);

  const listBox = await list.boundingBox();
  const selectedBox = await selected.boundingBox();
  expect(listBox).not.toBeNull();
  expect(selectedBox).not.toBeNull();
  if (!listBox || !selectedBox) return;

  expect(selectedBox.y).toBeGreaterThanOrEqual(listBox.y);
  expect(selectedBox.y + selectedBox.height).toBeLessThanOrEqual(listBox.y + listBox.height);

  // If the list is manually scrolled away from the selection, the next selection update should
  // re-scroll the selected item into view.
  await list.evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await page.evaluate(
    ({ formulaId, label }) => {
      const panel = document.querySelector(
        `[data-testid="completion-panel"][data-formula-id="${formulaId}"]`,
      );
      if (!panel) throw new Error("Missing completion panel");
      const items = panel.querySelectorAll<HTMLElement>(".completion-item");
      const target = Array.from(items).find(
        (item) => item.querySelector(".completion-item-label")?.textContent?.trim() === label,
      );
      if (!target) throw new Error(`Missing completion item: ${label}`);
      target.dispatchEvent(new MouseEvent("mouseenter", { bubbles: true }));
    },
    { formulaId: FORMULA_ID, label: targetLabel },
  );

  const listBox2 = await list.boundingBox();
  const selectedBox2 = await selected.boundingBox();
  expect(listBox2).not.toBeNull();
  expect(selectedBox2).not.toBeNull();
  if (!listBox2 || !selectedBox2) return;
  expect(selectedBox2.y).toBeGreaterThanOrEqual(listBox2.y);
  expect(selectedBox2.y + selectedBox2.height).toBeLessThanOrEqual(listBox2.y + listBox2.height);
});

test("cursor remains correct after multiple completion-driven edits", async ({ page }) => {
  await setEditorContent(page, FORMULA_ID, "sub");
  await waitForAnyCompletionItems(page, FORMULA_ID);
  await waitForCompletionDebounce(page);
  await applyCompletionByDomClick(page, FORMULA_ID, "substring");
  await expectEditorText(page, FORMULA_ID, "substring()");
  await expectCursorAfter(page, FORMULA_ID, "substring(");

  // Apply another completion later in the same document, ensuring cursor placement stays correct.
  await page.keyboard.press("End");
  await page.keyboard.type(" + re");
  await page.keyboard.press("ArrowLeft");
  await waitForAnyCompletionItems(page, FORMULA_ID);
  await waitForCompletionDebounce(page);
  await applyCompletionByDomClick(page, FORMULA_ID, "replace");
  await expectEditorText(page, FORMULA_ID, /substring\(\) \+ replace\(\)/);
  await expectCursorAfter(page, FORMULA_ID, " + replace(");

  await page.keyboard.press("End");
  await page.keyboard.type(" + su");
  await page.keyboard.press("ArrowLeft");
  await waitForAnyCompletionItems(page, FORMULA_ID);
  await waitForCompletionDebounce(page);
  await applyCompletionByDomClick(page, FORMULA_ID, "substring");
  await expectEditorText(page, FORMULA_ID, /replace\(\) \+ substring\(\)/);
  await expectCursorAfter(page, FORMULA_ID, "replace() + substring(");
});

test("cursor remains correct when formula contains UTF-16 characters", async ({ page }) => {
  await setEditorContent(page, FORMULA_ID, "汉字 + i");
  await waitForAnyCompletionItems(page, FORMULA_ID);
  await waitForCompletionDebounce(page);

  await applyCompletionByDomClick(page, FORMULA_ID, "if");
  await expectEditorText(page, FORMULA_ID, "汉字 + if()");

  await expectCursorAfter(page, FORMULA_ID, "if(");
});
