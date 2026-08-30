import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { checkDocumentation } from "./check-docs.mjs";

// Nested objects are directories; strings are file contents.
const fixtures = [
  {
    name: "accepts a valid bilingual repository",
    repository: {
      docs: {
        "example.md": `---
doc_id: architecture.example
title: "Example"
language: en
source_language: en
counterpart: ./example.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# Example

[简体中文](example.zh-CN.md)
[Architecture diagram](../assets/architecture.svg)
`,
        "example.zh-CN.md": `---
doc_id: architecture.example
title: "示例"
language: zh-CN
source_language: en
counterpart: ./example.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# 示例

[English](example.md)
`,
      },
      assets: {
        "architecture.svg": "<svg></svg>\n",
      },
    },
    expected: {
      documentCount: 2,
      errors: [],
      pairCount: 1,
    },
  },
  {
    name: "reports bilingual metadata drift",
    repository: {
      docs: {
        "example.md": `---
doc_id: architecture.example
title: "Example"
language: en
source_language: en
counterpart: ./example.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# Example

[简体中文](example.zh-CN.md)
`,
        "example.zh-CN.md": `---
doc_id: architecture.example
title: "示例"
language: zh-CN
source_language: en
counterpart: ./example.md
implementation_status: current
document_status: stable
translation_status: needs-update
last_verified: 2026-08-30
---

# 示例
`,
      },
    },
    expected: {
      documentCount: 2,
      errors: [
        "docs/example.md: translation_status does not match docs/example.zh-CN.md",
        "docs/example.zh-CN.md: document body must link to its counterpart",
        "docs/example.zh-CN.md: translation_status does not match docs/example.md",
      ],
      pairCount: 1,
    },
  },
  {
    name: "reports broken local links",
    repository: {
      "README.md": `# Example

[Missing](docs/missing.md)
[External](https://example.com)
[Heading](#example)

\`\`\`markdown
[Illustrative](docs/not-real.md)
\`\`\`
`,
      examples: {
        vite: {
          src: {
            pkg: {
              "README.md": "[Generated link](missing.md)\n",
            },
          },
        },
      },
    },
    expected: {
      documentCount: 1,
      errors: ["README.md:3: missing local link target docs/missing.md"],
      pairCount: 0,
    },
  },
  {
    name: "reports incomplete and invalid metadata",
    repository: {
      docs: {
        "example.md": `---
doc_id: architecture.example
language: english
translation_status: done
---

# Example
`,
      },
    },
    expected: {
      documentCount: 1,
      errors: [
        "doc_id architecture.example: expected exactly two language counterparts, found 1",
        "docs/example.md: invalid translation_status done; expected one of needs-update, synced",
        "docs/example.md: language must be en or zh-CN, got english",
        "docs/example.md: missing metadata field counterpart",
        "docs/example.md: missing metadata field document_status",
        "docs/example.md: missing metadata field implementation_status",
        "docs/example.md: missing metadata field last_verified",
        "docs/example.md: missing metadata field source_language",
        "docs/example.md: missing metadata field title",
      ],
      pairCount: 0,
    },
  },
];

function materializeRepositoryTree(directory, tree) {
  mkdirSync(directory, { recursive: true });
  for (const [name, entry] of Object.entries(tree)) {
    const entryPath = join(directory, name);
    if (typeof entry === "string") writeFileSync(entryPath, entry);
    else materializeRepositoryTree(entryPath, entry);
  }
}

function checkFixtureRepository(repository) {
  const repositoryRoot = mkdtempSync(join(tmpdir(), "check-docs-"));
  try {
    materializeRepositoryTree(repositoryRoot, repository);
    return checkDocumentation(repositoryRoot);
  } finally {
    rmSync(repositoryRoot, { force: true, recursive: true });
  }
}

for (const fixture of fixtures) {
  test(fixture.name, () => {
    const actual = checkFixtureRepository(fixture.repository);

    assert.deepEqual(actual, fixture.expected);
  });
}
