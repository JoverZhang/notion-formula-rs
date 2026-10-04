import { mkdtemp, readFile, rm } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";
import { build } from "vite";
import type {} from "../../src/formula/browser_contract";

let directory: string;
let server: Server;
let url: string;

test.beforeAll(async () => {
  directory = await mkdtemp(join(tmpdir(), "formula-worker-contract-"));
  const root = fileURLToPath(new URL("../..", import.meta.url));
  await build({
    configFile: false,
    root,
    base: "/",
    logLevel: "warn",
    build: {
      outDir: directory,
      emptyOutDir: true,
      rollupOptions: { input: join(root, "src/formula/contract.html") },
    },
  });
  const mime: Record<string, string> = {
    ".html": "text/html",
    ".js": "text/javascript",
    ".wasm": "application/wasm",
  };
  server = createServer((request, response) => {
    const path = new URL(request.url ?? "/", "http://localhost").pathname;
    const file = join(directory, path);
    void readFile(file)
      .then((body) => {
        response.writeHead(200, {
          "Content-Type": mime[extname(file)] ?? "application/octet-stream",
        });
        response.end(body);
      })
      .catch(() => {
        response.writeHead(404);
        response.end();
      });
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("Contract server must have a port");
  url = `http://127.0.0.1:${address.port}/src/formula/contract.html`;
});

test.afterAll(async () => {
  if (server)
    await new Promise<void>((resolve, reject) =>
      server.close((error) => (error ? reject(error) : resolve())),
    );
  if (directory) await rm(directory, { recursive: true, force: true });
});

test("real module Worker matches the synchronous WASM session contract", async ({ page }) => {
  await page.goto(url);
  await page.waitForFunction(() => Boolean(window.__formula_worker_contract));
  const result = await page.evaluate(() => window.__formula_worker_contract());
  expect(result).toEqual({
    verified: [
      "engine snapshots",
      "lossless evaluation DTOs",
      "typed native errors",
      "multiple drafts and borrow errors",
      "UTF-16 edits and versions",
      "consume discard and explicit commit",
      "request snapshots",
      "queued close and idempotence",
    ],
    rows: 4,
    specialNumbers: true,
    exactDate: true,
    maps: true,
    utf16Cursor: 2,
  });
});

test("real Worker initialization and execution failures release workers and settle Promises", async ({
  page,
}) => {
  await page.goto(url);
  await page.waitForFunction(() => Boolean(window.__formula_worker_failures));
  const result = await page.evaluate(() => window.__formula_worker_failures());
  expect(result).toEqual({ initTerminations: 1, failureTerminations: 1, settled: 2 });
});

test("diagnostic IDs remain scoped to their Worker client and current draft revision", async ({
  page,
}) => {
  await page.goto(url);
  await page.waitForFunction(() => Boolean(window.__formula_diagnostic_scopes));
  const result = await page.evaluate(() => window.__formula_diagnostic_scopes());
  expect(result).toEqual({
    distinct: true,
    stable: true,
    crossClientEmpty: true,
    staleEmpty: true,
    updateStateScoped: true,
  });
});
