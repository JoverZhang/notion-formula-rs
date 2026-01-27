import { platform } from "process";
import { expect, type Locator, type Page } from "@playwright/test";
import type { FormulaId } from "../../src/app/types";
import "../../src/debug/common";

const SELECT_ALL = platform === "darwin" ? "Meta+A" : "Control+A";

export async function gotoDebug(page: Page) {
  await page.goto("/?debug=1");
  await page.waitForFunction(() => Boolean(window.__nf_debug), null, { timeout: 10_000 });
}

export function editorContentLocator(page: Page, id: FormulaId): Locator {
  return page.locator(`[data-testid="formula-editor"][data-formula-id="${id}"] .cm-content`);
}

export function completionItemsLocator(page: Page, id: FormulaId): Locator {
  return page.locator(
    `[data-testid="completion-panel"][data-formula-id="${id}"] .completion-items`,
  );
}

export async function setEditorContent(page: Page, id: FormulaId, content: string) {
  const editor = editorContentLocator(page, id);
  await editor.click();
  await page.keyboard.press(SELECT_ALL);
  await page.keyboard.press("Backspace");
  if (content) {
    await page.keyboard.type(content);
  }
}

export async function waitForAnyCompletionItems(page: Page, id: FormulaId) {
  const items = completionItemsLocator(page, id).locator(".completion-item");
  await expect(items.first()).toBeVisible({ timeout: 5_000 });
}

export async function waitForCompletionDebounce(page: Page, ms = 200) {
  // The UI debounces completion requests (currently 120ms). Keep this slightly higher so tests
  // don't accidentally apply a completion computed for an earlier cursor position.
  await page.waitForTimeout(ms);
}

export async function clickCompletionByLabel(page: Page, id: FormulaId, label: RegExp | string) {
  const item = completionItemsLocator(page, id).locator(".completion-item", {
    has: page.locator(".completion-item-label", { hasText: label }),
  });
  await expect(item.first()).toBeVisible({ timeout: 5_000 });
  await item.first().click();
}

export async function expectCursorAt(page: Page, id: FormulaId, expectedHead: number) {
  await page.waitForFunction(
    ({ formulaId, expected }) => {
      const dbg = window.__nf_debug;
      if (!dbg) return false;
      const head = dbg.getSelectionHead(formulaId);
      return head === expected;
    },
    { formulaId: id, expected: expectedHead },
    { timeout: 5_000 },
  );
}

export async function expectCursorAfter(page: Page, id: FormulaId, needle: string) {
  const expected = await page.evaluate(
    ({ formulaId, needleText }) => {
      const editor = document.querySelector<HTMLElement>(
        `[data-testid="formula-editor"][data-formula-id="${formulaId}"] .cm-content`,
      );
      const source = editor?.textContent ?? "";
      const idx = source.indexOf(needleText);
      return idx === -1 ? null : idx + needleText.length;
    },
    { formulaId: id, needleText: needle },
  );

  if (typeof expected !== "number") {
    throw new Error(`Expected editor text to contain ${JSON.stringify(needle)}`);
  }

  await expect
    .poll(
      async () =>
        page.evaluate((formulaId) => window.__nf_debug?.getSelectionHead(formulaId) ?? null, id),
      { timeout: 5_000 },
    )
    .toBe(expected);
}

export async function applyCompletionByLabel(page: Page, id: FormulaId, label: RegExp | string) {
  const item = completionItemsLocator(page, id).locator(".completion-item", {
    has: page.locator(".completion-item-label", { hasText: label }),
  });
  await expect(item.first()).toBeVisible({ timeout: 5_000 });
  await item.first().hover();
  await page.keyboard.press("Enter");
}

export async function applyCompletionByDomClick(page: Page, id: FormulaId, label: string) {
  await page.waitForFunction(
    ({ formulaId, itemLabel }) => {
      const panel = document.querySelector(
        `[data-testid="completion-panel"][data-formula-id="${formulaId}"]`,
      );
      if (!panel) return false;
      const items = panel.querySelectorAll<HTMLElement>(".completion-item");
      return Array.from(items).some(
        (el) => el.querySelector(".completion-item-label")?.textContent?.trim() === itemLabel,
      );
    },
    { formulaId: id, itemLabel: label },
    { timeout: 5_000 },
  );

  await page.evaluate(
    ({ formulaId, itemLabel }) => {
      const panel = document.querySelector(
        `[data-testid="completion-panel"][data-formula-id="${formulaId}"]`,
      );
      if (!panel) throw new Error("Missing completion panel");
      const items = panel.querySelectorAll<HTMLElement>(".completion-item");
      const el = Array.from(items).find(
        (item) => item.querySelector(".completion-item-label")?.textContent?.trim() === itemLabel,
      );
      if (!el) throw new Error(`Missing completion item: ${itemLabel}`);
      el.click();
    },
    { formulaId: id, itemLabel: label },
  );
}

export async function expectEditorText(page: Page, id: FormulaId, expected: string | RegExp) {
  const editor = editorContentLocator(page, id);
  await expect(editor).toHaveText(expected, { timeout: 5_000 });
}
