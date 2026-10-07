import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const check = process.argv.length === 3 && process.argv[2] === "--check";
if (process.argv.length > 2 && !check) {
  throw new Error("Usage: node scripts/generate-wasm-client.mjs [--check]");
}

const root = new URL("../", import.meta.url);
const sourcePath = "docs/specs/wasm-api.md";
const targetPath = "packages/notion-formula/src/client.h.ts";
const source = await readFile(new URL(sourcePath, root), "utf8");
const blocks = [...source.matchAll(
  /^```ts spec-file=packages\/notion-formula\/src\/client\.h\.ts\r?\n([\s\S]*?)^```[ \t]*\r?$/gm,
)];
if (blocks.length !== 1) {
  throw new Error(`Expected exactly one client declaration block in ${sourcePath}`);
}
const generated = "// AUTO-GENERATED: node scripts/generate-wasm-client.mjs\n"
  + `// Source: ${sourcePath}\n\n`
  + blocks[0][1].replace(/\r\n/g, "\n");
const target = new URL(targetPath, root);
if (check) {
  let existing;
  try {
    existing = await readFile(target, "utf8");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  if (existing !== generated) {
    throw new Error(`${targetPath} is stale; run node scripts/generate-wasm-client.mjs`);
  }
} else {
  await mkdir(dirname(fileURLToPath(target)), { recursive: true });
  await writeFile(target, generated);
}
