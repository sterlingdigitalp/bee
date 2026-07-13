import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);

async function render() {
  const workerUrl = new URL("../dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);
  return worker.fetch(
    new Request("https://bridgevoice.test/", { headers: { accept: "text/html", host: "bridgevoice.test", "x-forwarded-host": "bridgevoice.test", "x-forwarded-proto": "https" } }),
    { ASSETS: { fetch: async () => new Response("Not found", { status: 404 }) } },
    { waitUntil() {}, passThroughOnException() {} },
  );
}

test("server-renders the complete BridgeVoice product experience", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);
  const html = await response.text();

  assert.match(html, /<title>BridgeVoice: Vibe Code With Your Voice/);
  assert.match(html, /Vibe code at the/);
  assert.match(html, /Hold\. Speak\. Release\. Land\./);
  assert.match(html, /Thirty seconds, sound on\./);
  assert.match(html, /Under the hood\./);
  assert.match(html, /One subscription\./);
  assert.match(html, /Frequently asked\./);
  assert.match(html, /Stop typing\./);
  assert.match(html, /BridgeVoice\.dmg/);
  assert.match(html, /BridgeVoice-setup\.exe/);
  assert.match(html, /BridgeVoice\.AppImage/);
  assert.match(html, /https:\/\/bridgevoice\.test\/og\.png/);
  assert.doesNotMatch(html, /codex-preview|Your site is taking shape|SkeletonPreview/);
});

test("ships all local product and social assets without starter remnants", async () => {
  await Promise.all([
    access(new URL("public/images/bridgevoice-icon.svg", root)),
    access(new URL("public/media/bridgevoice-promo.webp", root)),
    access(new URL("public/media/bridgevoice-film.webp", root)),
    access(new URL("public/og.png", root)),
  ]);

  const [page, layout, pkg] = await Promise.all([
    readFile(new URL("app/page.tsx", root), "utf8"),
    readFile(new URL("app/layout.tsx", root), "utf8"),
    readFile(new URL("package.json", root), "utf8"),
  ]);
  assert.match(page, /aria-label="BridgeVoice demo"/);
  assert.match(page, /<details key=\{q\}>/);
  assert.match(layout, /generateMetadata/);
  assert.doesNotMatch(pkg, /react-loading-skeleton/);
  await assert.rejects(access(new URL("app/_sites-preview/SkeletonPreview.tsx", root)));
});
