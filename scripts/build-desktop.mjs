import { spawnSync } from "node:child_process";

const env = { ...process.env };

if (process.platform === "darwin" && !env.APPLE_SIGNING_IDENTITY) {
  const result = spawnSync("security", ["find-identity", "-v", "-p", "codesigning"], {
    encoding: "utf8",
  });
  const identities = [...(result.stdout ?? "").matchAll(/^\s*\d+\)\s+[A-F0-9]+\s+"([^"]+)"/gm)]
    .map((match) => match[1]);
  const identity = identities.find((value) => value.startsWith("Developer ID Application:"))
    ?? identities.find((value) => value.startsWith("Apple Development:"));

  if (!identity) {
    console.error(
      "Bee requires a certificate-backed macOS signature so Accessibility permission survives updates. "
      + "Install an Apple Development or Developer ID Application certificate, or set APPLE_SIGNING_IDENTITY.",
    );
    process.exit(1);
  }

  env.APPLE_SIGNING_IDENTITY = identity;
  console.log(`Signing Bee with ${identity}`);
}

const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const build = spawnSync(
  npm,
  ["run", "tauri", "--", "build", ...process.argv.slice(2)],
  { env, stdio: "inherit" },
);

process.exit(build.status ?? 1);
