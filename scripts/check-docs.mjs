#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, extname, isAbsolute, posix, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const MANIFEST_PATH = "docs/manifest.toml";
const MANIFEST_CATEGORIES = [
  "bilingual_files",
  "bilingual_directories",
  "english_only_files",
  "neutral_redirect_files",
  "control_files",
  "ignored_directories",
];
const EXACT_CATEGORIES = [
  "english_only_files",
  "neutral_redirect_files",
  "control_files",
];
const TOOL_IGNORED_DIRECTORY_NAMES = new Set([
  ".agent",
  ".git",
  ".worktrees",
  "dist",
  "node_modules",
  "target",
]);
const TOOL_IGNORED_PATHS = new Set(["examples/vite/src/pkg"]);
const BILINGUAL_METADATA = [
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
const ENGLISH_ONLY_METADATA = [
  "doc_id",
  "title",
  "language",
  "implementation_status",
  "document_status",
  "last_verified",
];
const SHARED_BILINGUAL_METADATA = BILINGUAL_METADATA.filter(
  (field) => !["title", "language", "counterpart"].includes(field),
);
const ALLOWED_VALUES = {
  implementation_status: ["current", "planned", "exploratory", "deprecated", "historical"],
  document_status: ["draft", "stable"],
  translation_status: ["pending", "needs-update", "synced"],
};
const APPROVED_IGNORED_DIRECTORIES = [".agent"];

function displayPath(repositoryRoot, filePath) {
  return relative(repositoryRoot, filePath).split(sep).join("/");
}

function isInside(repositoryRoot, filePath) {
  const path = relative(repositoryRoot, filePath);
  return path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path);
}

function stripTomlComments(contents) {
  let result = "";
  let quote = null;
  let escaped = false;

  for (let index = 0; index < contents.length; index += 1) {
    const character = contents[index];
    if (quote === '"' && character === "\\" && !escaped) {
      escaped = true;
      result += character;
      continue;
    }
    if (character === quote && !escaped) quote = null;
    else if (quote === null && (character === '"' || character === "'")) quote = character;

    if (quote === null && character === "#") {
      while (index < contents.length && contents[index] !== "\n") index += 1;
      if (index < contents.length) result += "\n";
    } else {
      result += character;
    }
    escaped = false;
  }

  return result;
}

function parseTomlArray(arraySource, category) {
  const values = [];
  let index = 0;
  let needsValue = true;

  while (index < arraySource.length) {
    while (/\s/.test(arraySource[index] ?? "")) index += 1;
    if (index >= arraySource.length) break;

    if (!needsValue) {
      if (arraySource[index] !== ",") {
        throw new Error(`${category} must contain comma-separated strings`);
      }
      index += 1;
      needsValue = true;
      continue;
    }

    const quote = arraySource[index];
    if (quote !== '"' && quote !== "'") {
      throw new Error(`${category} must contain only quoted strings`);
    }

    const start = index;
    index += 1;
    let escaped = false;
    while (index < arraySource.length) {
      const character = arraySource[index];
      if (quote === '"' && character === "\\" && !escaped) {
        escaped = true;
        index += 1;
        continue;
      }
      if (character === quote && !escaped) break;
      escaped = false;
      index += 1;
    }
    if (index >= arraySource.length) throw new Error(`${category} contains an unclosed string`);

    const token = arraySource.slice(start, index + 1);
    const value = quote === '"' ? JSON.parse(token) : token.slice(1, -1);
    values.push(value);
    index += 1;
    needsValue = false;
  }

  return values;
}

function parseManifest(contents) {
  const source = stripTomlComments(contents).replaceAll("\r\n", "\n");
  const manifest = Object.fromEntries(MANIFEST_CATEGORIES.map((category) => [category, []]));
  const errors = [];
  const seenCategories = new Set();
  let index = 0;

  while (index < source.length) {
    while (/\s/.test(source[index] ?? "")) index += 1;
    if (index >= source.length) break;

    const keyMatch = source.slice(index).match(/^([A-Za-z_][A-Za-z0-9_]*)/);
    if (!keyMatch) {
      errors.push(`${MANIFEST_PATH}: unexpected TOML syntax`);
      break;
    }
    const category = keyMatch[1];
    index += keyMatch[0].length;
    while (/[ \t]/.test(source[index] ?? "")) index += 1;
    if (source[index] !== "=") {
      errors.push(`${MANIFEST_PATH}: expected = after ${category}`);
      break;
    }
    index += 1;
    while (/[ \t]/.test(source[index] ?? "")) index += 1;
    if (source[index] !== "[") {
      errors.push(`${MANIFEST_PATH}: ${category} must be an array`);
      break;
    }

    const arrayStart = index + 1;
    index += 1;
    let quote = null;
    let escaped = false;
    while (index < source.length) {
      const character = source[index];
      if (quote === '"' && character === "\\" && !escaped) {
        escaped = true;
        index += 1;
        continue;
      }
      if (character === quote && !escaped) quote = null;
      else if (quote === null && (character === '"' || character === "'")) quote = character;
      else if (quote === null && character === "]") break;
      escaped = false;
      index += 1;
    }
    if (index >= source.length) {
      errors.push(`${MANIFEST_PATH}: ${category} contains an unclosed array`);
      break;
    }

    if (!MANIFEST_CATEGORIES.includes(category)) {
      errors.push(`${MANIFEST_PATH}: unknown category ${category}`);
    } else if (seenCategories.has(category)) {
      errors.push(`${MANIFEST_PATH}: duplicate category ${category}`);
    } else {
      try {
        manifest[category] = parseTomlArray(source.slice(arrayStart, index), category);
      } catch (error) {
        errors.push(`${MANIFEST_PATH}: ${error.message}`);
      }
      seenCategories.add(category);
    }
    index += 1;
    while (/[ \t]/.test(source[index] ?? "")) index += 1;
    if (index < source.length && source[index] !== "\n") {
      errors.push(`${MANIFEST_PATH}: unexpected syntax after ${category}`);
      break;
    }
  }

  for (const category of MANIFEST_CATEGORIES) {
    if (!seenCategories.has(category)) {
      errors.push(`${MANIFEST_PATH}: missing category ${category}`);
    }
  }

  return { errors, manifest };
}

function validateManifestPaths(manifest) {
  const errors = [];

  for (const category of MANIFEST_CATEGORIES) {
    const seenPaths = new Set();
    for (const path of manifest[category]) {
      if (typeof path !== "string" || path.length === 0) {
        errors.push(`${MANIFEST_PATH}: ${category} contains an empty path`);
        continue;
      }
      if (
        path.includes("\\") ||
        isAbsolute(path) ||
        path.startsWith("./") ||
        path.endsWith("/") ||
        posix.normalize(path) !== path ||
        path === "." ||
        path.startsWith("../")
      ) {
        errors.push(`${MANIFEST_PATH}: ${category} contains invalid repository path ${path}`);
      }
      if (/[*?[\]{}]/.test(path)) {
        errors.push(`${MANIFEST_PATH}: ${category} must use exact paths, got ${path}`);
      }
      if (seenPaths.has(path)) {
        errors.push(`${MANIFEST_PATH}: ${category} repeats ${path}`);
      }
      seenPaths.add(path);

      if (category.endsWith("_files") && extname(path).toLowerCase() !== ".md") {
        errors.push(`${MANIFEST_PATH}: ${category} entry must be a Markdown file: ${path}`);
      }
      if (category === "bilingual_files" && path.endsWith(".zh-CN.md")) {
        errors.push(`${MANIFEST_PATH}: bilingual_files must use the base .md path: ${path}`);
      }
    }
  }

  if (
    manifest.ignored_directories.length !== APPROVED_IGNORED_DIRECTORIES.length ||
    manifest.ignored_directories.some(
      (directory, index) => directory !== APPROVED_IGNORED_DIRECTORIES[index],
    )
  ) {
    errors.push(`${MANIFEST_PATH}: ignored_directories must contain exactly .agent`);
  }

  return errors;
}

function loadDocumentationManifest(repositoryRoot) {
  const manifestPath = resolve(repositoryRoot, MANIFEST_PATH);
  if (!existsSync(manifestPath)) {
    return {
      errors: [`${MANIFEST_PATH}: documentation manifest is missing`],
      manifest: null,
    };
  }

  const parsed = parseManifest(readFileSync(manifestPath, "utf8"));
  parsed.errors.push(...validateManifestPaths(parsed.manifest));
  return parsed;
}

function pathIsWithinDirectory(path, directory) {
  return path === directory || path.startsWith(`${directory}/`);
}

function shouldIgnoreDirectory(repositoryRoot, directoryPath, directoryName) {
  const path = displayPath(repositoryRoot, directoryPath);
  return (
    TOOL_IGNORED_DIRECTORY_NAMES.has(directoryName) ||
    TOOL_IGNORED_PATHS.has(path)
  );
}

function parseFrontmatter(repositoryRoot, filePath, contents) {
  const lines = contents
    .replace(/^\uFEFF/, "")
    .replaceAll("\r\n", "\n")
    .split("\n");
  if (lines[0] !== "---") {
    return { body: lines.join("\n"), errors: [], frontmatterPresent: false, metadata: null };
  }

  const closingIndex = lines.indexOf("---", 1);
  if (closingIndex === -1) {
    return {
      body: lines.join("\n"),
      errors: [`${displayPath(repositoryRoot, filePath)}: unclosed YAML frontmatter`],
      frontmatterPresent: true,
      metadata: null,
    };
  }

  const errors = [];
  const metadata = {};
  for (const [offset, line] of lines.slice(1, closingIndex).entries()) {
    if (line.trim() === "" || line.trimStart().startsWith("#")) continue;
    const match = line.match(/^([A-Za-z_][A-Za-z0-9_-]*):\s*(.*?)\s*$/);
    if (!match) {
      errors.push(
        `${displayPath(repositoryRoot, filePath)}:${offset + 2}: invalid YAML metadata line`,
      );
      continue;
    }

    const [, field, rawValue] = match;
    if (field in metadata) {
      errors.push(`${displayPath(repositoryRoot, filePath)}: duplicate metadata field ${field}`);
      continue;
    }
    metadata[field] = rawValue.replace(/^(?:"(.*)"|'(.*)')$/, "$1$2");
  }

  return {
    body: lines.slice(closingIndex + 1).join("\n"),
    errors,
    frontmatterPresent: true,
    metadata,
  };
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

function discoverRepositoryDocumentation(repositoryRoot) {
  const documents = [];

  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const entryPath = resolve(directory, entry.name);
      if (
        entry.isDirectory() &&
        shouldIgnoreDirectory(repositoryRoot, entryPath, entry.name)
      ) {
        continue;
      }

      if (entry.isDirectory()) {
        visit(entryPath);
      } else if (entry.isFile() && extname(entry.name).toLowerCase() === ".md") {
        const parsed = parseFrontmatter(
          repositoryRoot,
          entryPath,
          readFileSync(entryPath, "utf8"),
        );
        documents.push({ ...parsed, filePath: entryPath, links: extractLinks(parsed.body) });
      }
    }
  }

  visit(repositoryRoot);
  return documents.sort((left, right) => left.filePath.localeCompare(right.filePath));
}

function bilingualBasePath(path) {
  return path.endsWith(".zh-CN.md") ? path.replace(/\.zh-CN\.md$/, ".md") : path;
}

function bilingualCounterpartPath(path) {
  return path.endsWith(".zh-CN.md")
    ? path.replace(/\.zh-CN\.md$/, ".md")
    : path.replace(/\.md$/, ".zh-CN.md");
}

function classifyDocumentation(repositoryRoot, manifest, documents) {
  const errors = [];
  const documentPaths = new Set(
    documents.map((document) => displayPath(repositoryRoot, document.filePath)),
  );
  const exactMatches = new Map();

  for (const category of EXACT_CATEGORIES) {
    for (const path of manifest[category]) {
      const categories = exactMatches.get(path) ?? [];
      categories.push(category);
      exactMatches.set(path, categories);
      if (!documentPaths.has(path)) {
        errors.push(`${MANIFEST_PATH}: ${category} references missing Markdown file ${path}`);
      }
    }
  }

  for (const [path, categories] of exactMatches) {
    if (categories.length > 1) {
      errors.push(
        `${MANIFEST_PATH}: ${path} appears in multiple exact categories: ${categories.join(", ")}`,
      );
    }
  }

  for (const path of manifest.bilingual_files) {
    const counterpart = bilingualCounterpartPath(path);
    if (!documentPaths.has(path) && !documentPaths.has(counterpart)) {
      errors.push(`${MANIFEST_PATH}: bilingual_files references missing document pair ${path}`);
    }
  }

  const classifiedDocuments = documents.map((document) => {
    const path = displayPath(repositoryRoot, document.filePath);
    const exact = exactMatches.get(path);
    if (exact?.length) return { ...document, category: exact[0], path };

    const basePath = bilingualBasePath(path);
    if (
      manifest.bilingual_files.includes(basePath) ||
      manifest.bilingual_directories.some((directory) => pathIsWithinDirectory(path, directory))
    ) {
      return { ...document, category: "bilingual", path };
    }

    errors.push(`${path}: Markdown file is not classified by ${MANIFEST_PATH}`);
    return { ...document, category: "unclassified", path };
  });

  return { documents: classifiedDocuments, errors };
}

function checkMetadataValues(path, metadata) {
  const errors = [];
  for (const field of ["language", "source_language"]) {
    const value = metadata[field];
    if (value && !["en", "zh-CN"].includes(value)) {
      errors.push(`${path}: ${field} must be en or zh-CN, got ${value}`);
    }
  }

  for (const [field, allowed] of Object.entries(ALLOWED_VALUES)) {
    const value = metadata[field];
    if (value && !allowed.includes(value)) {
      errors.push(`${path}: invalid ${field} ${value}; expected one of ${allowed.join(", ")}`);
    }
  }

  const date = metadata.last_verified;
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

function checkRequiredMetadata(document, requiredFields) {
  if (document.metadata === null) {
    return [`${document.path}: required YAML metadata is missing`];
  }

  const errors = [];
  for (const field of requiredFields) {
    if (!document.metadata[field]) errors.push(`${document.path}: missing metadata field ${field}`);
  }
  errors.push(...checkMetadataValues(document.path, document.metadata));
  return errors;
}

function resolveDeclaredCounterpart(repositoryRoot, document) {
  const declared = document.metadata?.counterpart;
  if (!declared) return null;
  const counterpart = resolve(dirname(document.filePath), declared);
  return isInside(repositoryRoot, counterpart) ? counterpart : false;
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

function checkBilingualDocumentation(repositoryRoot, documents) {
  const errors = [];
  const translationDebt = [];
  const groups = new Map();
  const docIds = new Map();
  let pairCount = 0;

  for (const document of documents) {
    errors.push(...document.errors, ...checkRequiredMetadata(document, BILINGUAL_METADATA));
    const basePath = bilingualBasePath(document.path);
    const group = groups.get(basePath) ?? [];
    group.push(document);
    groups.set(basePath, group);

    if (document.metadata?.doc_id) {
      const owners = docIds.get(document.metadata.doc_id) ?? new Set();
      owners.add(basePath);
      docIds.set(document.metadata.doc_id, owners);
    }

    if (document.metadata?.language === "en" && document.path.endsWith(".zh-CN.md")) {
      errors.push(`${document.path}: language en must use the base .md path`);
    }
    if (document.metadata?.language === "zh-CN" && !document.path.endsWith(".zh-CN.md")) {
      errors.push(`${document.path}: language zh-CN must use the .zh-CN.md path`);
    }

    const declaredPath = resolveDeclaredCounterpart(repositoryRoot, document);
    const expectedPath = resolve(repositoryRoot, bilingualCounterpartPath(document.path));
    if (declaredPath === false) {
      errors.push(`${document.path}: counterpart escapes the repository`);
    } else if (declaredPath && declaredPath !== expectedPath) {
      errors.push(`${document.path}: counterpart must use the adjacent .zh-CN.md convention`);
    }
  }

  for (const [docId, owners] of docIds) {
    if (owners.size > 1) {
      errors.push(
        `doc_id ${docId}: used by multiple bilingual documents: ${[...owners].join(", ")}`,
      );
    }
  }

  for (const [basePath, group] of groups) {
    const byPath = new Map(group.map((document) => [document.path, document]));
    const source = byPath.get(basePath);
    const translation = byPath.get(bilingualCounterpartPath(basePath));
    const existing = source ?? translation;
    const status = existing?.metadata?.translation_status;

    if (!source || !translation) {
      const expected = bilingualCounterpartPath(existing.path);
      if (status === "pending") {
        if (existing.metadata?.language !== existing.metadata?.source_language) {
          errors.push(`${existing.path}: pending document must be written in its source_language`);
        }
        translationDebt.push(`${existing.path}: counterpart ${expected} is pending`);
      } else {
        errors.push(`${existing.path}: required counterpart ${expected} is missing`);
        if (status === "needs-update") {
          translationDebt.push(`${basePath}: translation needs update`);
        }
      }
      continue;
    }

    pairCount += 1;
    if (
      source.metadata?.translation_status === "pending" ||
      translation.metadata?.translation_status === "pending"
    ) {
      errors.push(`${basePath}: pending is only valid while the counterpart is absent`);
    }

    const languages = [source.metadata?.language, translation.metadata?.language].sort().join(",");
    if (languages !== "en,zh-CN") {
      errors.push(`${basePath}: pair must contain one en and one zh-CN document`);
    }

    for (const field of SHARED_BILINGUAL_METADATA) {
      if (
        source.metadata?.[field] &&
        translation.metadata?.[field] &&
        source.metadata[field] !== translation.metadata[field]
      ) {
        errors.push(`${basePath}: ${field} does not match ${translation.path}`);
      }
    }

    for (const [document, counterpart] of [
      [source, translation],
      [translation, source],
    ]) {
      const declared = resolveDeclaredCounterpart(repositoryRoot, document);
      if (declared !== counterpart.filePath) {
        errors.push(`${document.path}: counterpart metadata is not reciprocal`);
      }
      const linkedTargets = document.links.map((link) =>
        resolveLocalLink(document.filePath, link.destination),
      );
      if (!linkedTargets.includes(counterpart.filePath)) {
        errors.push(`${document.path}: document body must link to its counterpart`);
      }
    }

    if (
      source.metadata?.translation_status === "needs-update" ||
      translation.metadata?.translation_status === "needs-update"
    ) {
      translationDebt.push(`${basePath}: translation needs update`);
    }
  }

  return { errors, pairCount, translationDebt };
}

function checkEnglishOnlyDocumentation(documents) {
  const errors = [];
  for (const document of documents) {
    errors.push(...document.errors, ...checkRequiredMetadata(document, ENGLISH_ONLY_METADATA));
    if (document.metadata?.language && document.metadata.language !== "en") {
      errors.push(`${document.path}: English-only document must use language en`);
    }
  }
  return errors;
}

function targetIsIgnored(repositoryRoot, targetPath) {
  const path = displayPath(repositoryRoot, targetPath);
  return (
    path.split("/").some((segment) => TOOL_IGNORED_DIRECTORY_NAMES.has(segment)) ||
    [...TOOL_IGNORED_PATHS].some((directory) => pathIsWithinDirectory(path, directory))
  );
}

function isPendingCounterpart(document, targetPath) {
  if (document.category !== "bilingual" || document.metadata?.translation_status !== "pending") {
    return false;
  }
  const expected = resolve(dirname(document.filePath), document.metadata.counterpart ?? "");
  return expected === targetPath;
}

function checkLocalLinks(repositoryRoot, documents) {
  const errors = [];
  for (const document of documents) {
    for (const link of document.links) {
      const target = resolveLocalLink(document.filePath, link.destination);
      const prefix = `${document.path}:${link.line}`;
      if (target === false) {
        errors.push(`${prefix}: invalid URL encoding in ${link.destination}`);
      } else if (target && !isInside(repositoryRoot, target)) {
        errors.push(`${prefix}: local link escapes the repository: ${link.destination}`);
      } else if (
        target &&
        !targetIsIgnored(repositoryRoot, target) &&
        !existsSync(target) &&
        !isPendingCounterpart(document, target)
      ) {
        errors.push(`${prefix}: missing local link target ${link.destination}`);
      }
    }
  }
  return errors;
}

function validateDocumentation(repositoryRoot, classification, manifestErrors) {
  const { documents, errors: classificationErrors } = classification;
  const errors = [...manifestErrors, ...classificationErrors];
  const bilingual = checkBilingualDocumentation(
    repositoryRoot,
    documents.filter((document) => document.category === "bilingual"),
  );

  errors.push(...bilingual.errors);
  errors.push(
    ...checkEnglishOnlyDocumentation(
      documents.filter((document) => document.category === "english_only_files"),
    ),
  );

  for (const document of documents.filter((candidate) =>
    ["neutral_redirect_files", "control_files"].includes(candidate.category),
  )) {
    if (document.frontmatterPresent) {
      errors.push(`${document.path}: ${document.category} must not use YAML frontmatter`);
    }
  }

  errors.push(...checkLocalLinks(repositoryRoot, documents));

  return {
    documentCount: documents.length,
    errors: [...new Set(errors)].sort(),
    pairCount: bilingual.pairCount,
    translationDebt: [...new Set(bilingual.translationDebt)].sort(),
  };
}

export function checkDocumentation(repositoryRoot) {
  // Produces one classified documentation report for the repository.
  const root = resolve(repositoryRoot);

  const manifestResult = loadDocumentationManifest(root);
  if (manifestResult.manifest === null) {
    return {
      documentCount: 0,
      errors: manifestResult.errors,
      pairCount: 0,
      translationDebt: [],
    };
  }

  const documents = discoverRepositoryDocumentation(root);

  const classification = classifyDocumentation(root, manifestResult.manifest, documents);

  return validateDocumentation(
    root,
    classification,
    manifestResult.errors,
  );
}

function reportDebt(title, entries) {
  if (entries.length === 0) return;
  console.log(`${title} (${entries.length}):`);
  for (const entry of entries) console.log(`- ${entry}`);
}

function reportDocumentationResult(report) {
  if (report.errors.length === 0) {
    console.log(
      `Documentation checks passed: ${report.documentCount} Markdown files, ` +
        `${report.pairCount} bilingual pairs.`,
    );
  } else {
    console.error(`Documentation checks failed (${report.errors.length}):`);
    for (const error of report.errors) console.error(`- ${error}`);
    process.exitCode = 1;
  }

  reportDebt("Translation debt", report.translationDebt);
}

function main() {
  // Validates all maintained Markdown and reports structural errors and translation debt.
  const report = checkDocumentation(process.cwd());

  reportDocumentationResult(report);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
