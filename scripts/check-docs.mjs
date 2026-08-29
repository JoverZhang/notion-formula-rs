#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, extname, isAbsolute, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const IGNORED_DIRECTORIES = new Set([
  ".agent",
  ".git",
  ".worktrees",
  "dist",
  "node_modules",
  "target",
]);
const IGNORED_PATHS = new Set(["examples/vite/src/pkg"]);
const REQUIRED_METADATA = [
  "doc_id",
  "title",
  "language",
  "source_language",
  "counterpart",
  "implementation_status",
  "document_status",
  "translation_status",
  "last_verified",
];
const SHARED_METADATA = REQUIRED_METADATA.filter(
  (field) => !["title", "language", "counterpart"].includes(field),
);
const ALLOWED_VALUES = {
  implementation_status: ["current", "planned", "exploratory", "deprecated", "historical"],
  document_status: ["draft", "stable"],
  translation_status: ["needs-update", "synced"],
};

function displayPath(repositoryRoot, filePath) {
  return relative(repositoryRoot, filePath).split(sep).join("/");
}

function isInside(repositoryRoot, filePath) {
  const path = relative(repositoryRoot, filePath);
  return path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path);
}

function discoverMarkdownFiles(repositoryRoot) {
  const files = [];

  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const entryPath = resolve(directory, entry.name);
      if (
        entry.isDirectory() &&
        (IGNORED_DIRECTORIES.has(entry.name) ||
          IGNORED_PATHS.has(displayPath(repositoryRoot, entryPath)))
      ) {
        continue;
      }

      if (entry.isDirectory()) visit(entryPath);
      else if (entry.isFile() && extname(entry.name).toLowerCase() === ".md") files.push(entryPath);
    }
  }

  visit(repositoryRoot);
  return files.sort();
}

function parseFrontmatter(repositoryRoot, filePath, contents) {
  const lines = contents
    .replace(/^\uFEFF/, "")
    .replaceAll("\r\n", "\n")
    .split("\n");
  if (lines[0] !== "---") return { body: lines.join("\n"), errors: [], metadata: null };

  const closingIndex = lines.indexOf("---", 1);
  if (closingIndex === -1) {
    return {
      body: lines.join("\n"),
      errors: [`${displayPath(repositoryRoot, filePath)}: unclosed YAML frontmatter`],
      metadata: null,
    };
  }

  const errors = [];
  const metadata = {};
  for (const line of lines.slice(1, closingIndex)) {
    const match = line.match(/^([A-Za-z_][A-Za-z0-9_-]*):\s*(.*?)\s*$/);
    if (!match || !REQUIRED_METADATA.includes(match[1])) continue;

    const [, field, rawValue] = match;
    if (field in metadata) {
      errors.push(`${displayPath(repositoryRoot, filePath)}: duplicate metadata field ${field}`);
    } else {
      metadata[field] = rawValue.replace(/^(?:"(.*)"|'(.*)')$/, "$1$2");
    }
  }

  return { body: lines.slice(closingIndex + 1).join("\n"), errors, metadata };
}

function extractLinks(contents) {
  const links = [];
  const withoutComments = contents.replaceAll(/<!--[\s\S]*?-->/g, (comment) =>
    comment.replaceAll(/[^\n]/g, " "),
  );
  let fence = null;

  for (const [index, originalLine] of withoutComments.split("\n").entries()) {
    const fenceMatch = originalLine.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (fenceMatch) {
      const marker = fenceMatch[1];
      if (fence === null) fence = marker[0];
      else if (marker[0] === fence) fence = null;
      continue;
    }
    if (fence !== null) continue;

    const line = originalLine.replaceAll(/(`+).*?\1/g, "");
    const inlinePattern = /!?\[[^\]]*\]\(\s*(?:<([^>]+)>|((?:\\.|[^()\s]|\([^)]*\))+))/g;
    for (const match of line.matchAll(inlinePattern)) {
      links.push({ destination: match[1] ?? match[2], line: index + 1 });
    }

    const reference = line.match(/^\s{0,3}\[[^\]]+\]:\s*(?:<([^>]+)>|(\S+))/);
    if (reference) links.push({ destination: reference[1] ?? reference[2], line: index + 1 });
  }

  return links;
}

function resolveLocalLink(documentPath, destination) {
  const unescaped = destination.replaceAll(/\\([\\()[\] ])/g, "$1");
  if (
    !unescaped ||
    unescaped.startsWith("#") ||
    unescaped.startsWith("/") ||
    unescaped.startsWith("//") ||
    /^[A-Za-z][A-Za-z0-9+.-]*:/.test(unescaped)
  ) {
    return null;
  }

  const path = unescaped.split(/[?#]/, 1)[0];
  if (!path) return null;

  try {
    return resolve(dirname(documentPath), decodeURIComponent(path));
  } catch {
    return false;
  }
}

function loadDocumentation(repositoryRoot) {
  return discoverMarkdownFiles(repositoryRoot).map((filePath) => {
    const parsed = parseFrontmatter(repositoryRoot, filePath, readFileSync(filePath, "utf8"));
    return { ...parsed, filePath, links: extractLinks(parsed.body) };
  });
}

function checkLocalLinks(repositoryRoot, documents) {
  const errors = [];
  for (const document of documents) {
    for (const link of document.links) {
      const target = resolveLocalLink(document.filePath, link.destination);
      const prefix = `${displayPath(repositoryRoot, document.filePath)}:${link.line}`;
      if (target === false) errors.push(`${prefix}: invalid URL encoding in ${link.destination}`);
      else if (target && !isInside(repositoryRoot, target)) {
        errors.push(`${prefix}: local link escapes the repository: ${link.destination}`);
      } else if (target && !existsSync(target)) {
        errors.push(`${prefix}: missing local link target ${link.destination}`);
      }
    }
  }
  return errors;
}

function checkMetadata(repositoryRoot, document) {
  if (document.metadata === null) return [];

  const errors = [];
  const path = displayPath(repositoryRoot, document.filePath);
  for (const field of REQUIRED_METADATA) {
    if (!document.metadata[field]) errors.push(`${path}: missing metadata field ${field}`);
  }

  for (const field of ["language", "source_language"]) {
    const value = document.metadata[field];
    if (value && !["en", "zh-CN"].includes(value)) {
      errors.push(`${path}: ${field} must be en or zh-CN, got ${value}`);
    }
  }

  for (const [field, allowed] of Object.entries(ALLOWED_VALUES)) {
    const value = document.metadata[field];
    if (value && !allowed.includes(value)) {
      errors.push(`${path}: invalid ${field} ${value}; expected one of ${allowed.join(", ")}`);
    }
  }

  const date = document.metadata.last_verified;
  if (date) {
    const parsed = new Date(`${date}T00:00:00Z`);
    if (
      !/^\d{4}-\d{2}-\d{2}$/.test(date) ||
      Number.isNaN(parsed.valueOf()) ||
      parsed.toISOString().slice(0, 10) !== date
    ) {
      errors.push(`${path}: last_verified must be a valid YYYY-MM-DD date, got ${date}`);
    }
  }
  return errors;
}

function checkBilingualPairs(repositoryRoot, documents) {
  const errors = documents.flatMap((document) => checkMetadata(repositoryRoot, document));
  const pairedDocuments = documents.filter((document) => document.metadata !== null);
  const byPath = new Map(documents.map((document) => [document.filePath, document]));
  const byId = new Map();
  const pairs = new Set();

  for (const document of pairedDocuments) {
    const matches = byId.get(document.metadata.doc_id) ?? [];
    matches.push(document);
    byId.set(document.metadata.doc_id, matches);
  }

  for (const [docId, matches] of byId) {
    if (docId && matches.length !== 2) {
      errors.push(
        `doc_id ${docId}: expected exactly two language counterparts, found ${matches.length}`,
      );
    }
  }

  for (const document of pairedDocuments) {
    const path = displayPath(repositoryRoot, document.filePath);
    if (!document.metadata.counterpart) continue;

    const counterpartPath = resolve(dirname(document.filePath), document.metadata.counterpart);
    if (!isInside(repositoryRoot, counterpartPath)) {
      errors.push(`${path}: counterpart escapes the repository`);
      continue;
    }

    const counterpart = byPath.get(counterpartPath);
    if (!counterpart?.metadata) {
      errors.push(`${path}: counterpart is missing or has no YAML metadata`);
      continue;
    }

    const expectedPath =
      document.metadata.language === "en"
        ? document.filePath.replace(/\.md$/, ".zh-CN.md")
        : document.filePath.replace(/\.zh-CN\.md$/, ".md");
    if (expectedPath !== counterpartPath) {
      errors.push(`${path}: counterpart does not follow the adjacent .zh-CN.md naming convention`);
    }

    const reciprocal = counterpart.metadata.counterpart
      ? resolve(dirname(counterpart.filePath), counterpart.metadata.counterpart)
      : null;
    if (reciprocal !== document.filePath)
      errors.push(`${path}: counterpart metadata is not reciprocal`);

    const languages = [document.metadata.language, counterpart.metadata.language].sort().join(",");
    if (languages !== "en,zh-CN")
      errors.push(`${path}: pair must contain one en and one zh-CN document`);

    for (const field of SHARED_METADATA) {
      if (document.metadata[field] !== counterpart.metadata[field]) {
        errors.push(
          `${path}: ${field} does not match ${displayPath(repositoryRoot, counterpart.filePath)}`,
        );
      }
    }

    const linkedTargets = document.links.map((link) =>
      resolveLocalLink(document.filePath, link.destination),
    );
    if (!linkedTargets.includes(counterpartPath))
      errors.push(`${path}: document body must link to its counterpart`);

    pairs.add([document.filePath, counterpartPath].sort().join("\0"));
  }

  return { errors, pairCount: pairs.size };
}

export function checkDocumentation(repositoryRoot) {
  const root = resolve(repositoryRoot);
  const documents = loadDocumentation(root);
  const bilingual = checkBilingualPairs(root, documents);
  const errors = [...documents.flatMap((document) => document.errors), ...bilingual.errors];

  errors.push(...checkLocalLinks(root, documents));
  return {
    documentCount: documents.length,
    errors: [...new Set(errors)].sort(),
    pairCount: bilingual.pairCount,
  };
}

function main() {
  // Validates repository documentation without changing files or external state.
  const report = checkDocumentation(process.cwd());
  if (report.errors.length === 0) {
    console.log(
      `Documentation checks passed: ${report.documentCount} Markdown files, ${report.pairCount} bilingual pairs.`,
    );
    return;
  }

  console.error("Documentation checks failed:");
  for (const error of report.errors) console.error(`- ${error}`);
  process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
