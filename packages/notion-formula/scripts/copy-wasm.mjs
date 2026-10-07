import { copyFile, mkdir } from "node:fs/promises";

const target = new URL("../dist/wasm/", import.meta.url);
await mkdir(target, { recursive: true });
for (const file of [
  "analyzer_wasm.js",
  "analyzer_wasm.d.ts",
  "analyzer_wasm_bg.wasm",
  "analyzer_wasm_bg.wasm.d.ts",
]) {
  await copyFile(new URL(`../src/wasm/${file}`, import.meta.url), new URL(file, target));
}
