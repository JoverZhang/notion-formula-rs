#!/usr/bin/env node

import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
if (process.argv.length > 3) {
  throw new Error("Usage: node scripts/stage-appflowy-sdk.mjs [appflowy-checkout]");
}

const appflowy = resolve(process.argv[2] ?? resolve(root, "examples/appflowy-web"));
const appManifest = resolve(appflowy, "package.json");
if (!existsSync(appManifest) || JSON.parse(readFileSync(appManifest, "utf8")).name !== "appflowy_web_app") {
  throw new Error("Initialize examples/appflowy-web, or pass an AppFlowy-Web checkout.");
}

const sdk = resolve(root, "packages/notion-formula");
const manifest = readFileSync(resolve(sdk, "package.json"), "utf8");
const name = JSON.parse(manifest).name;
for (const file of ["dist/client.js", "dist/client.d.ts", "dist/worker.js", "dist/wasm/analyzer_wasm_bg.wasm"]) {
  if (!existsSync(resolve(sdk, file))) {
    throw new Error(`Missing SDK artifact ${file}; run just wasm first.`);
  }
}

// AppFlowy links this local package and keeps its own lockfile. Copy only the
// distributable files so its Vite build cannot accidentally import SDK sources.
const destination = resolve(appflowy, ".notion-formula-sdk");
if (existsSync(destination)) {
  const previousManifest = resolve(destination, "package.json");
  if (!existsSync(previousManifest) || JSON.parse(readFileSync(previousManifest, "utf8")).name !== name) {
    throw new Error(`${destination} is not a staged ${name} package.`);
  }
  rmSync(destination, { recursive: true });
}
mkdirSync(destination);
writeFileSync(resolve(destination, "package.json"), manifest);
cpSync(resolve(sdk, "dist"), resolve(destination, "dist"), { recursive: true });
console.log(`Staged ${name} in ${destination}`);
