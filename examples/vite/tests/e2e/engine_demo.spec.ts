import { expect, test, type Page } from "@playwright/test";
import type { FormulaId } from "../../src/app/types";
import {
  applyCompletionByDomClick,
  completionItemsLocator,
  expectEditorText,
  gotoDebug,
  setEditorContent,
} from "./helpers";

const cells = (page: Page, id: FormulaId) =>
  page.locator(`[data-testid="formula-cell"][data-formula-id="${id}"]`);
const button = (page: Page, id: FormulaId, action: "save" | "discard") =>
  page.locator(`[data-testid="${action}-button"][data-formula-id="${id}"]`);

async function openReady(page: Page) {
  await gotoDebug(page);
  await expect(cells(page, "Formula 1")).toHaveText(["24", "14", "8", "36"]);
  await expect(cells(page, "Formula 2")).toHaveText(["25", "15", "9", "37"]);
}

async function save(page: Page, id: FormulaId, expression: string) {
  await setEditorContent(page, id, expression);
  await button(page, id, "save").click();
  await expect(button(page, id, "save")).toBeDisabled();
  await expect(page.locator(`[data-testid="formula-dirty"][data-formula-id="${id}"]`)).toBeHidden();
}

const propertyCompletion = (page: Page, id: FormulaId, label: string) =>
  completionItemsLocator(page, id).locator(".completion-item", {
    has: page.locator(".completion-item-label").filter({ hasText: new RegExp(`^${label}$`) }),
  });

test("formula display names are the engine property IDs used by completion edits", async ({
  page,
}) => {
  await openReady(page);
  await expectEditorText(page, "Formula 2", 'prop("Formula 1") + 1');
  await setEditorContent(page, "Formula 2", "For");
  const candidate = propertyCompletion(page, "Formula 2", "Formula 1");
  await expect(candidate).toBeVisible();
  await expect(candidate).not.toHaveClass(/is-disabled/);
  await applyCompletionByDomClick(page, "Formula 2", "Formula 1");
  await expectEditorText(page, "Formula 2", 'prop("Formula 1")');
  await expect(button(page, "Formula 2", "save")).toBeEnabled();
  await button(page, "Formula 2", "save").click();
  await expect(cells(page, "Formula 2")).toHaveText(["24", "14", "8", "36"]);

  await save(page, "Formula 2", 'prop("Formula 1") + 1');
  await expect(cells(page, "Formula 2")).toHaveText(["25", "15", "9", "37"]);
});

test("an unready formula stays visible with its reason and cannot be selected or applied", async ({
  page,
}) => {
  await openReady(page);
  await save(page, "Formula 1", "if(");
  await setEditorContent(page, "Formula 2", "");
  const list = completionItemsLocator(page, "Formula 2");
  const candidate = propertyCompletion(page, "Formula 2", "Formula 1");
  await expect(candidate).toBeVisible();
  await expect(candidate).toHaveAttribute("aria-disabled", "true");
  await expect(candidate).toContainText("Formula property `Formula 1` is not ready");
  await expect(candidate).not.toHaveAttribute("data-completion-recommended", "true");
  await expect(list.locator(".completion-group-header", { hasText: /^Properties$/ })).toBeVisible();
  await candidate.hover();
  await expect(candidate).not.toHaveClass(/is-selected/);
  await applyCompletionByDomClick(page, "Formula 2", "Formula 1");
  await expectEditorText(page, "Formula 2", "");
  const selectableCount = await list.locator(".completion-item:not(.is-disabled)").count();
  for (const key of ["ArrowDown", "ArrowUp"]) {
    for (let index = 0; index <= selectableCount; index += 1) {
      await page.keyboard.press(key);
      await expect(list.locator(".completion-item.is-selected")).not.toHaveClass(/is-disabled/);
    }
  }
  await expectEditorText(page, "Formula 2", "");
});

test("self and dependent formula candidates stay visible with dependency-cycle reasons", async ({
  page,
}) => {
  await openReady(page);
  await setEditorContent(page, "Formula 1", "");
  const list = completionItemsLocator(page, "Formula 1");
  for (const label of ["Formula 1", "Formula 2"]) {
    const candidate = propertyCompletion(page, "Formula 1", label);
    await expect(candidate).toBeVisible();
    await expect(candidate).toHaveAttribute("aria-disabled", "true");
    await expect(candidate).toContainText(
      `Referencing property \`${label}\` creates a dependency cycle`,
    );
    await expect(candidate).not.toHaveAttribute("data-completion-recommended", "true");
    await applyCompletionByDomClick(page, "Formula 1", label);
    await expectEditorText(page, "Formula 1", "");
  }
  await expect(list.locator(".completion-group-header", { hasText: /^Properties$/ })).toBeVisible();
});

test("full sample inputs produce typed results and shared dependencies", async ({ page }) => {
  await openReady(page);
  await expect(page.locator('th[data-property-id="Title"]')).toHaveText("Title");
  await expect(page.locator('td[data-property-id="Relation"]').first()).toHaveText(
    "[North Star, Blueprint]",
  );
  await expect(page.locator('th[data-formula-id="Formula 1"]')).toHaveAttribute(
    "data-output-type",
    "number",
  );

  await save(page, "Formula 1", 'prop("Number") * 3');
  await expect(cells(page, "Formula 1")).toHaveText(["36", "21", "12", "54"]);
  await expect(cells(page, "Formula 2")).toHaveText(["37", "22", "13", "55"]);
});

test("saving one draft preserves the other buffer; discard restores its saved source", async ({
  page,
}) => {
  await openReady(page);
  const unsaved = 'prop("Number") + 100';
  await setEditorContent(page, "Formula 2", unsaved);
  await expect(button(page, "Formula 2", "save")).toBeEnabled();
  await expect(cells(page, "Formula 2")).toHaveText(["25", "15", "9", "37"]);

  await save(page, "Formula 1", 'prop("Number") * 3');
  await expectEditorText(page, "Formula 2", unsaved);
  await expect(button(page, "Formula 2", "save")).toBeEnabled();
  await expect(cells(page, "Formula 2")).toHaveText(["37", "22", "13", "55"]);

  await button(page, "Formula 2", "discard").click();
  await expectEditorText(page, "Formula 2", 'prop("Formula 1") + 1');
  await expect(button(page, "Formula 2", "discard")).toBeDisabled();
  await expect(cells(page, "Formula 2")).toHaveText(["37", "22", "13", "55"]);
});

test("row errors retain their origin and ordinary nulls remain distinct", async ({ page }) => {
  await openReady(page);
  await save(page, "Formula 2", 'prop("Formula 1")');
  await save(page, "Formula 1", 'if(prop("Number") == 7, test("x", "["), true)');
  for (const id of ["Formula 1", "Formula 2"] as const) {
    await expect(cells(page, id)).toHaveText(["true", "Error", "true", "true"]);
    await expect(cells(page, id).nth(1)).toHaveAttribute("data-error-origins", "Formula 1");
    await expect(cells(page, id).nth(1)).toHaveAttribute(
      "title",
      /Formula 1: Invalid regular expression/,
    );
  }

  await save(page, "Formula 1", "empty()");
  for (const id of ["Formula 1", "Formula 2"] as const) {
    await expect(cells(page, id)).toHaveText(["null", "null", "null", "null"]);
    await expect(cells(page, id).nth(1)).not.toHaveClass(/is-error/);
    await expect(cells(page, id).nth(1)).not.toHaveAttribute("data-error-origins");
  }
});

test("special numbers and nested nulls survive Worker evaluation and rendering", async ({
  page,
}) => {
  await openReady(page);
  await save(page, "Formula 1", "[-0, 1 / 0, 0 / 0, empty(), [1, empty()]]");
  await expect(cells(page, "Formula 1")).toHaveText(
    Array(4).fill("[-0, Infinity, NaN, null, [1, null]]"),
  );
  await save(page, "Formula 1", "flat([[[1]], [[2]]])");
  await expect(cells(page, "Formula 1")).toHaveText(Array(4).fill("[[1], [2]]"));
  await expect(page.locator('th[data-formula-id="Formula 1"]')).toHaveAttribute(
    "data-output-type",
    "list<list<number>>",
  );
});

test("date inputs render dates and unrelated formulas survive a NotReady save", async ({
  page,
}) => {
  await openReady(page);
  await save(page, "Formula 2", "123");
  await save(page, "Formula 1", 'prop("Date")');
  await expect(cells(page, "Formula 1")).toHaveText([
    "2024-05-14",
    "2024-06-02",
    "2024-06-09",
    "2024-06-16",
  ]);
  await expect(page.locator('th[data-formula-id="Formula 1"]')).toHaveAttribute(
    "data-output-type",
    "date",
  );
  await save(page, "Formula 1", "if(");
  await expect(cells(page, "Formula 1")).toHaveText(Array(4).fill("Not ready"));
  await expect(cells(page, "Formula 2")).toHaveText(Array(4).fill("123"));
});

test("WASM initialization failure is visible and never leaves results looking current", async ({
  page,
}) => {
  await page.route("**/*.wasm", (route) => route.abort());
  await gotoDebug(page);
  await expect(page.locator('[data-testid="app-error"]')).toBeVisible();
  await expect(cells(page, "Formula 1")).toHaveText(Array(4).fill("Unavailable"));
  await expect(button(page, "Formula 1", "save")).toBeDisabled();
});

test("a cached page keeps its unsaved draft and resumes native commands", async ({ page }) => {
  await openReady(page);
  const expression = 'prop("Number") * 3';
  await setEditorContent(page, "Formula 1", expression);
  await expect(button(page, "Formula 1", "save")).toBeEnabled();
  // Playwright's default headless shell disables BFCache. Exercise the persisted
  // lifecycle branch explicitly; full Chromium also restores this branch on Back.
  await page.evaluate(() => {
    window.dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true }));
    window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
  });
  await expectEditorText(page, "Formula 1", expression);
  await expect(cells(page, "Formula 1")).toHaveText(["24", "14", "8", "36"]);
  await button(page, "Formula 1", "save").click();
  await expect(cells(page, "Formula 1")).toHaveText(["36", "21", "12", "54"]);
  await expect(cells(page, "Formula 2")).toHaveText(["37", "22", "13", "55"]);

  await page.evaluate(() => {
    window.dispatchEvent(new PageTransitionEvent("pagehide", { persisted: false }));
  });
  await expect(page.locator('[data-testid="formula-panel"]')).toHaveCount(0);
  expect(await page.evaluate(() => Boolean(window.__nf_debug))).toBe(false);
});
