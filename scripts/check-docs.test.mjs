import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import { checkDocumentation } from "./check-docs.mjs";

function writeDocument(repositoryRoot, path, contents) {
  const destination = join(repositoryRoot, path);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, contents);
}

function pairedMetadata({ counterpart, language, status = "synced" }) {
  return `---
doc_id: architecture.example
title: "Example"
language: ${language}
source_language: en
counterpart: ${counterpart}
implementation_status: current
document_status: stable
translation_status: ${status}
last_verified: 2026-08-29
---`;
}

function withRepository(run) {
  const repositoryRoot = mkdtempSync(join(tmpdir(), "check-docs-"));
  try {
    run(repositoryRoot);
  } finally {
    rmSync(repositoryRoot, { force: true, recursive: true });
  }
}

test("accepts reciprocal bilingual metadata and valid local links", () => {
  withRepository((repositoryRoot) => {
    writeDocument(
      repositoryRoot,
      "docs/example.md",
      `${pairedMetadata({ counterpart: "./example.zh-CN.md", language: "en" })}

# Example

[简体中文](example.zh-CN.md)
[Shared asset](../asset.txt)
`,
    );
    writeDocument(
      repositoryRoot,
      "docs/example.zh-CN.md",
      `${pairedMetadata({ counterpart: "./example.md", language: "zh-CN" })}

# 示例

[English](example.md)
`,
    );
    writeDocument(repositoryRoot, "asset.txt", "shared\n");

    const report = checkDocumentation(repositoryRoot);

    assert.deepEqual(report.errors, []);
    assert.equal(report.documentCount, 2);
    assert.equal(report.pairCount, 1);
  });
});

test("reports metadata drift and a missing reciprocal body link", () => {
  withRepository((repositoryRoot) => {
    writeDocument(
      repositoryRoot,
      "docs/example.md",
      `${pairedMetadata({ counterpart: "./example.zh-CN.md", language: "en" })}

# Example

[简体中文](example.zh-CN.md)
`,
    );
    writeDocument(
      repositoryRoot,
      "docs/example.zh-CN.md",
      `${pairedMetadata({ counterpart: "./example.md", language: "zh-CN", status: "needs-update" })}

# 示例
`,
    );

    const report = checkDocumentation(repositoryRoot);

    assert(report.errors.some((error) => error.includes("translation_status does not match")));
    assert(
      report.errors.some((error) => error.includes("document body must link to its counterpart")),
    );
  });
});

test("reports broken local links but ignores external, anchor, and fenced links", () => {
  withRepository((repositoryRoot) => {
    writeDocument(
      repositoryRoot,
      "README.md",
      `# Example

[Missing](docs/missing.md)
[External](https://example.com)
[Heading](#example)

\`\`\`markdown
[Illustrative](docs/not-real.md)
\`\`\`
`,
    );
    writeDocument(
      repositoryRoot,
      "examples/vite/src/pkg/README.md",
      "[Generated link](missing.md)\n",
    );

    const report = checkDocumentation(repositoryRoot);

    assert.deepEqual(report.errors, ["README.md:3: missing local link target docs/missing.md"]);
  });
});

test("reports incomplete and invalid bilingual metadata", () => {
  withRepository((repositoryRoot) => {
    writeDocument(
      repositoryRoot,
      "docs/example.md",
      `---
doc_id: architecture.example
language: english
translation_status: done
---

# Example
`,
    );

    const report = checkDocumentation(repositoryRoot);

    assert(report.errors.some((error) => error.includes("missing metadata field counterpart")));
    assert(report.errors.some((error) => error.includes("language must be en or zh-CN")));
    assert(report.errors.some((error) => error.includes("invalid translation_status done")));
  });
});
