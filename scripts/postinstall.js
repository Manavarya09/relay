#!/usr/bin/env node

const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const VERSION = "1.2.0";
const REPO = "Manavarya09/relay";

const PLATFORM_MAP = {
  "darwin-arm64": "relay-macos-arm64",
  "darwin-x64": "relay-macos-x86_64",
  "linux-x64": "relay-linux-x86_64",
  "win32-x64": "relay-windows-x86_64.exe",
};

const platform = `${process.platform}-${process.arch}`;
const asset = PLATFORM_MAP[platform];

if (!asset) {
  console.log(`  relay: No pre-built binary for ${platform}. Build from source.`);
  process.exit(0);
}

const binDir = path.join(__dirname, "..", "bin");
const ext = process.platform === "win32" ? ".exe" : "";
const binPath = path.join(binDir, `relay${ext}`);

// Skip if real binary already exists (not the node wrapper)
if (fs.existsSync(binPath)) {
  const content = fs.readFileSync(binPath, "utf8").slice(0, 50);
  if (!content.includes("node") && !content.includes("#!/usr/bin/env")) {
    process.exit(0); // Already have native binary
  }
}

fs.mkdirSync(binDir, { recursive: true });

const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;
const tmpPath = binPath + ".tmp";

console.log(`  relay: Downloading ${asset}...`);

try {
  execFileSync("curl", [
    "-sL", "--fail", "--max-time", "30",
    "-o", tmpPath,
    url,
  ], { stdio: ["pipe", "pipe", "pipe"] });

  fs.renameSync(tmpPath, binPath);
  fs.chmodSync(binPath, 0o755);
  console.log(`  relay: Installed.`);
} catch (e) {
  // Clean up tmp
  try { fs.unlinkSync(tmpPath); } catch {}
  console.log(`  relay: Download failed. Binary not available for ${platform}.`);
  console.log(`  relay: Build from source: git clone https://github.com/${REPO} && cd relay && ./scripts/build.sh`);

  // Write a helpful error script as fallback
  const fallback = `#!/bin/sh\necho "relay binary not installed. Run: npm run postinstall"\nexit 1\n`;
  fs.writeFileSync(binPath, fallback);
  fs.chmodSync(binPath, 0o755);
}
