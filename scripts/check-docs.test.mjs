import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { checkDocumentation } from "./check-docs.mjs";

const checkerPath = fileURLToPath(new URL("./check-docs.mjs", import.meta.url));

const manifestCategories = [
  "bilingual_files",
  "bilingual_directories",
  "english_only_files",
  "neutral_redirect_files",
  "control_files",
  "ignored_directories",
];
const defaultManifestEntries = {
  ignored_directories: [".agent"],
};

function documentationManifest(categories = {}) {
  return manifestCategories
    .map((category) => {
      const entries = categories[category] ?? defaultManifestEntries[category] ?? [];
      const values = entries.map((entry) => `  ${JSON.stringify(entry)},`).join("\n");
      return values ? `${category} = [\n${values}\n]` : `${category} = []`;
    })
    .join("\n\n");
}

function bilingualDocument({
  body,
  counterpart,
  docId = "docs.guide",
  language,
  lastVerified = "2026-08-30",
  sourceLanguage = "en",
  title,
  translationStatus = "synced",
}) {
  return `---
doc_id: ${docId}
title: "${title}"
language: ${language}
source_language: ${sourceLanguage}
counterpart: ${counterpart}
implementation_status: current
document_status: stable
translation_status: ${translationStatus}
last_verified: ${lastVerified}
---

${body}
`;
}

function englishOnlyDocument({ language = "en", lastVerified = "2026-08-30", title }) {
  return `---
doc_id: reference.glossary
title: "${title}"
language: ${language}
implementation_status: current
document_status: stable
last_verified: ${lastVerified}
---

# ${title}
`;
}

// Nested objects are directories; strings are the complete file contents.
const fixtures = [
  {
    name: "accepts every durable category and a synced bilingual pair",
    repository: {
      ".agent": {
        "legacy.md": "[This broken link is ignored](missing.md)\n",
      },
      "AGENTS.md": "Legacy material is ignored: [.agent](.agent/legacy.md).\n",
      "GLOSSARY.md": englishOnlyDocument({ title: "Glossary" }),
      analyzer: {
        "README.md": "[Implementation guide](../docs/guide.md)\n",
      },
      docs: {
        assets: {
          "architecture.svg": "<svg></svg>\n",
        },
        "guide.md": bilingualDocument({
          body: "# Guide\n\n[简体中文](guide.zh-CN.md)\n[Diagram](assets/architecture.svg)",
          counterpart: "./guide.zh-CN.md",
          language: "en",
          title: "Guide",
        }),
        "guide.zh-CN.md": bilingualDocument({
          body: "# 指南\n\n[English](guide.md)",
          counterpart: "./guide.md",
          language: "zh-CN",
          title: "指南",
        }),
        "manifest.toml": documentationManifest({
          bilingual_directories: ["docs"],
          control_files: ["AGENTS.md"],
          english_only_files: ["GLOSSARY.md"],
          ignored_directories: [".agent"],
          neutral_redirect_files: ["analyzer/README.md"],
        }),
      },
    },
    expected: {
      documentCount: 5,
      errors: [],
      pairCount: 1,
      translationDebt: [],
    },
  },
  {
    name: "allows an English source while its translation is pending",
    repository: {
      "README.md": bilingualDocument({
        body: "# Project\n\n[简体中文](README.zh-CN.md)",
        counterpart: "./README.zh-CN.md",
        docId: "project.readme",
        language: "en",
        title: "Project",
        translationStatus: "pending",
      }),
      docs: {
        "manifest.toml": documentationManifest({
          bilingual_files: ["README.md"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: [],
      pairCount: 0,
      translationDebt: ["README.md: counterpart README.zh-CN.md is pending"],
    },
  },
  {
    name: "allows a Chinese source while its translation is pending",
    repository: {
      "README.zh-CN.md": bilingualDocument({
        body: "# 项目\n\n[English](README.md)",
        counterpart: "./README.md",
        docId: "project.readme",
        language: "zh-CN",
        sourceLanguage: "zh-CN",
        title: "项目",
        translationStatus: "pending",
      }),
      docs: {
        "manifest.toml": documentationManifest({
          bilingual_files: ["README.md"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: [],
      pairCount: 0,
      translationDebt: ["README.zh-CN.md: counterpart README.md is pending"],
    },
  },
  {
    name: "reports a needs-update pair as non-failing translation debt",
    repository: {
      docs: {
        "guide.md": bilingualDocument({
          body: "# Guide\n\n[简体中文](guide.zh-CN.md)",
          counterpart: "./guide.zh-CN.md",
          language: "en",
          title: "Guide",
          translationStatus: "needs-update",
        }),
        "guide.zh-CN.md": bilingualDocument({
          body: "# 指南\n\n[English](guide.md)",
          counterpart: "./guide.md",
          language: "zh-CN",
          title: "指南",
          translationStatus: "needs-update",
        }),
        "manifest.toml": documentationManifest({
          bilingual_directories: ["docs"],
        }),
      },
    },
    expected: {
      documentCount: 2,
      errors: [],
      pairCount: 1,
      translationDebt: ["docs/guide.md: translation needs update"],
    },
  },
  {
    name: "skips an ignored legacy directory without traversing its Markdown",
    repository: {
      ".agent": {
        nested: {
          "legacy.md": "[Missing](nowhere.md)\n",
        },
      },
      docs: {
        "manifest.toml": documentationManifest({
          ignored_directories: [".agent"],
        }),
      },
    },
    expected: {
      documentCount: 0,
      errors: [],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "requires the manifest to declare the ignored legacy directory",
    repository: {
      ".agent": {
        nested: {
          "legacy.md": "[Missing](nowhere.md)\n",
        },
      },
      docs: {
        "manifest.toml": documentationManifest({
          ignored_directories: [],
        }),
      },
    },
    expected: {
      documentCount: 0,
      errors: ["docs/manifest.toml: ignored_directories must contain exactly .agent"],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "rejects an extra ignored directory without hiding its Markdown",
    repository: {
      hidden: {
        "notes.md": "# Notes\n",
      },
      docs: {
        "manifest.toml": documentationManifest({
          ignored_directories: [".agent", "hidden"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: [
        "docs/manifest.toml: ignored_directories must contain exactly .agent",
        "hidden/notes.md: Markdown file is not classified by docs/manifest.toml",
      ],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "rejects the retired legacy classification",
    repository: {
      docs: {
        "manifest.toml": `${documentationManifest()}\n\nlegacy_files = []`,
      },
    },
    expected: {
      documentCount: 0,
      errors: ["docs/manifest.toml: unknown category legacy_files"],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "rejects an unclassified Markdown file",
    repository: {
      "notes.md": "# Notes\n",
      docs: {
        "manifest.toml": documentationManifest(),
      },
    },
    expected: {
      documentCount: 1,
      errors: ["notes.md: Markdown file is not classified by docs/manifest.toml"],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "rejects a file listed in two exact categories",
    repository: {
      "README.md": "# Control document\n",
      docs: {
        "manifest.toml": documentationManifest({
          control_files: ["README.md"],
          neutral_redirect_files: ["README.md"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: [
        "docs/manifest.toml: README.md appears in multiple exact categories: " +
          "neutral_redirect_files, control_files",
      ],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "rejects TOML assignments joined on one line",
    repository: {
      docs: {
        "manifest.toml": documentationManifest().replace(
          "\n\nignored_directories",
          "ignored_directories",
        ),
      },
    },
    expected: {
      documentCount: 0,
      errors: [
        "docs/manifest.toml: ignored_directories must contain exactly .agent",
        "docs/manifest.toml: missing category ignored_directories",
        "docs/manifest.toml: unexpected syntax after control_files",
      ],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "rejects a TOML key split from its value",
    repository: {
      docs: {
        "manifest.toml": documentationManifest().replace(
          "ignored_directories = [",
          "ignored_directories\n= [",
        ),
      },
    },
    expected: {
      documentCount: 0,
      errors: [
        "docs/manifest.toml: expected = after ignored_directories",
        "docs/manifest.toml: ignored_directories must contain exactly .agent",
        "docs/manifest.toml: missing category ignored_directories",
      ],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "reports a missing synced counterpart",
    repository: {
      docs: {
        "guide.md": bilingualDocument({
          body: "# Guide\n\n[简体中文](guide.zh-CN.md)",
          counterpart: "./guide.zh-CN.md",
          language: "en",
          title: "Guide",
        }),
        "manifest.toml": documentationManifest({
          bilingual_directories: ["docs"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: [
        "docs/guide.md: required counterpart docs/guide.zh-CN.md is missing",
        "docs/guide.md:4: missing local link target guide.zh-CN.md",
      ],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "reports counterpart metadata that points outside the adjacent pair",
    repository: {
      docs: {
        "guide.md": bilingualDocument({
          body: "# Guide\n\n[简体中文](guide.zh-CN.md)",
          counterpart: "./other.zh-CN.md",
          language: "en",
          title: "Guide",
        }),
        "guide.zh-CN.md": bilingualDocument({
          body: "# 指南\n\n[English](guide.md)",
          counterpart: "./guide.md",
          language: "zh-CN",
          title: "指南",
        }),
        "manifest.toml": documentationManifest({
          bilingual_directories: ["docs"],
        }),
      },
    },
    expected: {
      documentCount: 2,
      errors: [
        "docs/guide.md: counterpart metadata is not reciprocal",
        "docs/guide.md: counterpart must use the adjacent .zh-CN.md convention",
      ],
      pairCount: 1,
      translationDebt: [],
    },
  },
  {
    name: "reports invalid English-only metadata",
    repository: {
      "GLOSSARY.md": englishOnlyDocument({
        language: "zh-CN",
        lastVerified: "2026-02-30",
        title: "Glossary",
      }),
      docs: {
        "manifest.toml": documentationManifest({
          english_only_files: ["GLOSSARY.md"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: [
        "GLOSSARY.md: English-only document must use language en",
        "GLOSSARY.md: last_verified must be a valid YYYY-MM-DD date, got 2026-02-30",
      ],
      pairCount: 0,
      translationDebt: [],
    },
  },
  {
    name: "reports a broken local link outside code examples",
    repository: {
      "README.md": `# Project

[Missing](docs/missing.md)

\`\`\`markdown
[Illustrative](docs/not-real.md)
\`\`\`
`,
      docs: {
        "manifest.toml": documentationManifest({
          control_files: ["README.md"],
        }),
      },
    },
    expected: {
      documentCount: 1,
      errors: ["README.md:3: missing local link target docs/missing.md"],
      pairCount: 0,
      translationDebt: [],
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

function withFixtureRepository(repository, operation) {
  const repositoryRoot = mkdtempSync(join(tmpdir(), "check-docs-"));
  try {
    materializeRepositoryTree(repositoryRoot, repository);
    return operation(repositoryRoot);
  } finally {
    rmSync(repositoryRoot, { force: true, recursive: true });
  }
}

function checkFixtureRepository(repository) {
  return withFixtureRepository(repository, (repositoryRoot) =>
    checkDocumentation(repositoryRoot),
  );
}

function runChecker(repository) {
  return withFixtureRepository(repository, (repositoryRoot) =>
    spawnSync(process.execPath, [checkerPath], {
      cwd: repositoryRoot,
      encoding: "utf8",
    }),
  );
}

for (const fixture of fixtures) {
  test(fixture.name, () => {
    const actual = checkFixtureRepository(fixture.repository);

    assert.deepEqual(actual, fixture.expected);
  });
}

test("CLI exits nonzero when documentation has structural errors", () => {
  const result = runChecker({
    "notes.md": "# Unclassified notes\n",
    docs: {
      "manifest.toml": documentationManifest(),
    },
  });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Documentation checks failed \(1\)/);
});

test("CLI exits zero while reporting translation debt", () => {
  const result = runChecker({
    "README.md": bilingualDocument({
      body: "# Project\n\n[简体中文](README.zh-CN.md)",
      counterpart: "./README.zh-CN.md",
      docId: "project.readme",
      language: "en",
      title: "Project",
      translationStatus: "pending",
    }),
    docs: {
      "manifest.toml": documentationManifest({
        bilingual_files: ["README.md"],
      }),
    },
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /Translation debt \(1\)/);
  assert.doesNotMatch(result.stdout, /Migration debt/);
});
