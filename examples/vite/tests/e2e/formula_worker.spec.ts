import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type TestInfo } from "@playwright/test";
import { build, createServer as createViteServer, type ViteDevServer } from "vite";
import type {} from "../../src/formula/browser_contract";

let directory: string;
let server: Server;
let url: string;
let devServer: ViteDevServer;
let devUrl: string;

async function recordContract(testInfo: TestInfo, name: string, data: unknown): Promise<void> {
  await mkdir(testInfo.outputDir, { recursive: true });
  const path = testInfo.outputPath(name);
  await writeFile(path, JSON.stringify(data, null, 2));
  await testInfo.attach(name, { path, contentType: "application/json" });
}

test.beforeAll(async () => {
  directory = await mkdtemp(join(tmpdir(), "formula-worker-contract-"));
  const root = join(directory, "consumer");
  await mkdir(root);
  const demo = fileURLToPath(new URL("../..", import.meta.url));
  const sdk = fileURLToPath(new URL("../../../../packages/notion-formula", import.meta.url));
  const installed = join(root, "node_modules/@notion-formula/sdk");
  await mkdir(installed, { recursive: true });
  // Install the same files included in the package, without access to its source tree.
  await cp(join(sdk, "package.json"), join(installed, "package.json"));
  await cp(join(sdk, "dist"), join(installed, "dist"), { recursive: true });
  await cp(join(demo, "src/formula"), join(root, "src/formula"), { recursive: true });
  devServer = await createViteServer({
    configFile: false,
    root,
    base: "/sdk-consumer/",
    logLevel: "warn",
    optimizeDeps: { exclude: ["@notion-formula/sdk"] },
    server: { host: "127.0.0.1", port: 0 },
  });
  await devServer.listen();
  const devAddress = devServer.httpServer?.address();
  if (!devAddress || typeof devAddress === "string") throw new Error("Dev server must have a port");
  devUrl = `http://127.0.0.1:${devAddress.port}/sdk-consumer/src/formula/contract.html`;
  await build({
    configFile: false,
    root,
    base: "/sdk-consumer/",
    logLevel: "warn",
    build: {
      outDir: join(directory, "production"),
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
    const file = join(directory, "production", path.replace(/^\/sdk-consumer\//, ""));
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
  url = `http://127.0.0.1:${address.port}/sdk-consumer/src/formula/contract.html`;
});

test.afterAll(async () => {
  if (devServer) await devServer.close();
  if (server)
    await new Promise<void>((resolve, reject) =>
      server.close((error) => (error ? reject(error) : resolve())),
    );
  if (directory) await rm(directory, { recursive: true, force: true });
});

test("SDK string codec matches native evaluation and Worker tokens retain only raw fields", async ({
  page,
}, testInfo) => {
  await page.goto(url);
  await page.waitForFunction(() => Boolean(window.__formula_string_codec));
  const result = await page.evaluate(() => window.__formula_string_codec());
  expect(result.roundtrips).toHaveLength(13);
  expect(result.rejected).toHaveLength(10);
  expect(result.lexicalRejected).toHaveLength(16);
  expect(result.incompleteCall.tokens.map((token) => token.kind)).toEqual([
    "Ident",
    "OpenParen",
    "String",
    "Eof",
  ]);
  expect(result.propertyRoundtrips).toEqual([
    { source: String.raw`prop("\\q")`, propertyId: "\\q", value: 73 },
    { source: 'prop("q")', propertyId: "q", value: 41 },
  ]);
  expect(result.synchronous).toBe(true);
  expect(result.rawTokenFields).toBe(true);
  await recordContract(testInfo, "sdk-string-codec.json", result);
});

test("real module Worker matches the synchronous WASM session contract", async ({
  page,
}, testInfo) => {
  await page.goto(url);
  await page.waitForFunction(() => Boolean(window.__formula_worker_contract));
  const result = await page.evaluate(() => window.__formula_worker_contract());
  await recordContract(testInfo, "sdk-worker-contract.json", { base: "/sdk-consumer/", ...result });
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

test("installed package default Worker loads in Vite dev with a deployment base", async ({
  page,
}, testInfo) => {
  const assets: { url: string; status: number }[] = [];
  page.on("response", (response) => {
    if (response.url().includes("/dist/worker.js") || response.url().endsWith(".wasm")) {
      assets.push({ url: response.url(), status: response.status() });
    }
  });
  await page.goto(devUrl);
  await page.waitForFunction(() => Boolean(window.__formula_worker_contract));
  const result = await page.evaluate(() => window.__formula_worker_contract());
  expect(result.maps).toBe(true);
  expect(result.exactDate).toBe(true);
  expect(result.verified).toContain("queued close and idempotence");
  expect(assets.some((asset) => asset.url.endsWith(".wasm"))).toBe(true);
  expect(
    assets.every((asset) => asset.status === 200 && asset.url.includes("/sdk-consumer/")),
  ).toBe(true);
  await recordContract(testInfo, "sdk-dev-consumer.json", { ...result, assets });
});
