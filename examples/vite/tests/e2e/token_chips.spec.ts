import { expect, test } from "@playwright/test";
import { expectEditorText, gotoDebug, setEditorContent } from "./helpers";

test("chips require complete direct property calls in real WASM tokens", async ({
  page,
}, testInfo) => {
  await gotoDebug(page);
  const observations = [];
  for (const { source, tokenCount, chipCount } of [
    { source: 'prop("Title") +', tokenCount: 5, chipCount: 1 },
    { source: '"Title".prop("Title")', tokenCount: 6, chipCount: 0 },
    { source: 'prop("Title"', tokenCount: 3, chipCount: 0 },
    { source: String.raw`prop("Ti\qtle")`, tokenCount: 4, chipCount: 0 },
    { source: "prop(1)", tokenCount: 4, chipCount: 0 },
    { source: '"😀" + prop("Title")', tokenCount: 6, chipCount: 1 },
    { source: String.raw`prop("\Title")`, tokenCount: 4, chipCount: 1 },
    { source: String.raw`prop("\\Title")`, tokenCount: 4, chipCount: 0 },
    { source: 'prop("Title', tokenCount: 2, chipCount: 0 },
  ]) {
    await setEditorContent(page, "Formula 1", source);
    await expectEditorText(page, "Formula 1", source);
    await expect
      .poll(() => page.evaluate(() => window.__nf_debug?.getState("Formula 1").tokenCount))
      .toBe(tokenCount);
    await expect(
      page.locator('[data-testid="prop-chip"][data-formula-id="Formula 1"]'),
    ).toHaveCount(chipCount);
    const ranges = await page.evaluate(() => window.__nf_debug?.getChipUiRanges("Formula 1"));
    observations.push({ source, ranges });
  }
  expect(observations[0].ranges).toMatchObject([{ from: 0, to: 13, propName: "Title" }]);
  expect(observations[5].ranges).toMatchObject([{ from: 7, to: 20, propName: "Title" }]);
  expect(observations[6].ranges).toMatchObject([{ from: 0, to: 14, propName: "Title" }]);
  const screenshot = testInfo.outputPath("token-chips.png");
  await page.screenshot({ path: screenshot, fullPage: true });
  await testInfo.attach("token-chips.png", { path: screenshot, contentType: "image/png" });
  await testInfo.attach("token-chips.json", {
    body: JSON.stringify(observations, null, 2),
    contentType: "application/json",
  });
});
