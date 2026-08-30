import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { checkDocumentation } from "./check-docs.mjs";

const fixturesRoot = fileURLToPath(new URL("./fixtures/check-docs", import.meta.url));
const fixtures = readdirSync(fixturesRoot, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .sort((left, right) => left.name.localeCompare(right.name));

for (const fixture of fixtures) {
  test(`fixture: ${fixture.name}`, () => {
    const fixtureRoot = join(fixturesRoot, fixture.name);
    const repositoryRoot = join(fixtureRoot, "repository");
    const expected = JSON.parse(readFileSync(join(fixtureRoot, "expected.json"), "utf8"));

    assert.deepEqual(checkDocumentation(repositoryRoot), expected);
  });
}
