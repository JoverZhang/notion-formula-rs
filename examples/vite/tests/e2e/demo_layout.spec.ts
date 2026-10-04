import { expect, test } from "@playwright/test";
import { editorContentLocator, gotoDebug, setEditorContent } from "./helpers";

const LONG_OUTPUT_SOURCE = 'if(prop("Number") > 10, [1, "x", true, prop("Date")], [[[1, "x"]]])';

for (const width of [1280, 768, 390, 320]) {
  test(`long output type is fully readable at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await gotoDebug(page);
    await setEditorContent(page, "Formula 1", LONG_OUTPUT_SOURCE);
    await expect
      .poll(() => page.evaluate(() => window.__nf_debug?.getState("Formula 1").outputType))
      .toContain("list<");

    const expectedOutput = await page.evaluate(
      () => `output: ${window.__nf_debug!.getState("Formula 1").outputType}`,
    );
    const panel = page.locator('[data-testid="formula-panel"][data-formula-id="Formula 1"]');
    const output = panel.locator('[data-testid="formula-output-type"]');
    await expect(output).toHaveText(expectedOutput);
    expect(expectedOutput.length).toBeGreaterThan(70);

    const geometry = await panel.evaluate((element) => {
      const output = element.querySelector<HTMLElement>(".formula-output-type")!;
      const value = output.querySelector<HTMLElement>(".formula-output-type-value")!;
      const actions = element.querySelector<HTMLElement>(".formula-actions")!;
      const range = document.createRange();
      range.selectNodeContents(value);
      const textRects = Array.from(range.getClientRects());
      const outputRect = output.getBoundingClientRect();
      const actionRect = actions.getBoundingClientRect();
      return {
        belowActions: outputRect.top >= actionRect.bottom,
        outputWidth: outputRect.width,
        actionWidth: actionRect.width,
        clientWidth: value.clientWidth,
        scrollWidth: value.scrollWidth,
        lineCount: new Set(textRects.map((rect) => rect.top)).size,
        textContained: textRects.every(
          (rect) =>
            rect.left >= outputRect.left - 1 &&
            rect.right <= outputRect.right + 1 &&
            rect.top >= outputRect.top - 1 &&
            rect.bottom <= outputRect.bottom + 1,
        ),
        documentWidth: document.documentElement.scrollWidth,
        viewportWidth: document.documentElement.clientWidth,
      };
    });
    expect(geometry.belowActions).toBe(true);
    expect(Math.abs(geometry.outputWidth - geometry.actionWidth)).toBeLessThan(1);
    expect(geometry.scrollWidth).toBeLessThanOrEqual(geometry.clientWidth + 1);
    if (width === 1280 || width <= 390) expect(geometry.lineCount).toBeGreaterThan(1);
    expect(geometry.textContained).toBe(true);
    expect(geometry.documentWidth).toBeLessThanOrEqual(geometry.viewportWidth + 1);
  });
}

for (const width of [1280, 390]) {
  test(`diagnostics stay inside their panel and allow editor switching at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await gotoDebug(page);
    await setEditorContent(page, "Formula 1", 'prop("Formula 2") + 1');
    const panel = page.locator('[data-testid="formula-panel"][data-formula-id="Formula 1"]');
    const signature = panel.locator('[data-testid="suggestion-signature"]');
    const diagnostics = panel.locator('[data-testid="formula-diagnostics"]');
    await expect(diagnostics).toBeVisible();
    await expect(diagnostics).toContainText(/cycl/i);
    const panelBox = await panel.boundingBox();
    const signatureBox = await signature.boundingBox();
    expect(panelBox).not.toBeNull();
    expect(signatureBox).not.toBeNull();
    expect(signatureBox!.x).toBeGreaterThanOrEqual(panelBox!.x);
    expect(signatureBox!.x + signatureBox!.width).toBeLessThanOrEqual(
      panelBox!.x + panelBox!.width,
    );
    expect(signatureBox!.y).toBeGreaterThanOrEqual(panelBox!.y);
    expect(signatureBox!.y + signatureBox!.height).toBeLessThanOrEqual(
      panelBox!.y + panelBox!.height,
    );

    const secondEditor = editorContentLocator(page, "Formula 2");
    await secondEditor.click();
    await expect(secondEditor).toBeFocused();
    await expect(signature).toBeHidden();
    await expect
      .poll(() =>
        page.evaluate(
          () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
        ),
      )
      .toBeLessThanOrEqual(1);
  });
}

for (const width of [1280, 390, 320]) {
  test(`long signature wraps inside its panel at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await gotoDebug(page);
    await setEditorContent(page, "Formula 1", LONG_OUTPUT_SOURCE);
    await page.evaluate(() => {
      const debug = window.__nf_debug!;
      const id = "Formula 1";
      const source = debug.getState(id).source;
      const rawPos = source.lastIndexOf('"x"') + '"x"'.length;
      debug.setSelectionHead(id, rawPos);
    });
    const panel = page.locator('[data-testid="formula-panel"][data-formula-id="Formula 1"]');
    const signature = panel.locator('[data-testid="suggestion-signature"]');
    const main = signature.locator(".completion-signature-main");
    await expect(main).toBeVisible();
    await expect(signature).toHaveAttribute("data-wrap", "wrapped");
    await expect(main).toContainText("condition: boolean");
    await expect
      .poll(() => main.evaluate((element) => element.scrollWidth - element.clientWidth))
      .toBeLessThanOrEqual(1);
    const panelBox = await panel.boundingBox();
    const signatureBox = await signature.boundingBox();
    expect(panelBox).not.toBeNull();
    expect(signatureBox).not.toBeNull();
    expect(signatureBox!.x).toBeGreaterThanOrEqual(panelBox!.x);
    expect(signatureBox!.x + signatureBox!.width).toBeLessThanOrEqual(
      panelBox!.x + panelBox!.width,
    );
    await expect
      .poll(() =>
        page.evaluate(
          () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
        ),
      )
      .toBeLessThanOrEqual(1);
  });
}

test("visible signature rewraps on resize without moving the cursor", async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 900 });
  await gotoDebug(page);
  const source = 'if(true, 1, "x")';
  const cursor = source.lastIndexOf('"x"') + '"x"'.length;
  await setEditorContent(page, "Formula 1", source);
  await page.evaluate((cursor) => window.__nf_debug!.setSelectionHead("Formula 1", cursor), cursor);

  const signature = page.locator(
    '[data-testid="suggestion-signature"][data-formula-id="Formula 1"]',
  );
  const main = signature.locator(".completion-signature-main");
  await expect(signature).toHaveAttribute("data-wrap", "unwrapped");
  await expect(main).toContainText("condition: boolean");

  for (const [width, mode] of [
    [320, "wrapped"],
    [900, "unwrapped"],
  ] as const) {
    await page.setViewportSize({ width, height: 900 });
    await expect(signature).toBeVisible();
    await expect(signature).toHaveAttribute("data-wrap", mode);
    await expect
      .poll(() => main.evaluate((element) => element.scrollWidth - element.clientWidth))
      .toBeLessThanOrEqual(1);
    await expect
      .poll(() => page.evaluate(() => window.__nf_debug!.getSelectionHead("Formula 1")))
      .toBe(cursor);
    await expect
      .poll(() => page.evaluate(() => window.__nf_debug!.getState("Formula 1").source))
      .toBe(source);
  }
});
