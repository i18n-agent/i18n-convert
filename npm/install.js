#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const os = require("os");

const VERSION = require("./package.json").version;
const REPO = "i18n-agent/i18n-convert";

const PLATFORM_MAP = {
  darwin: {
    x64: "x86_64-apple-darwin",
    arm64: "aarch64-apple-darwin",
  },
  linux: {
    x64: "x86_64-unknown-linux-gnu",
    arm64: "aarch64-unknown-linux-gnu",
  },
  win32: {
    x64: "x86_64-pc-windows-msvc",
  },
};

function getTarget() {
  const platform = os.platform();
  const arch = os.arch();

  const platformTargets = PLATFORM_MAP[platform];
  if (!platformTargets) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  const target = platformTargets[arch];
  if (!target) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }

  return target;
}

function getDownloadUrl(target) {
  const ext = target.includes("windows") ? "zip" : "tar.gz";
  return `https://github.com/${REPO}/releases/download/v${VERSION}/i18n-convert-${target}.${ext}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (res.statusCode === 302 || res.statusCode === 301) {
          return download(res.headers.location).then(resolve).catch(reject);
        }
        if (res.statusCode !== 200) {
          return reject(
            new Error(`Download failed with status ${res.statusCode}: ${url}`)
          );
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function extractTarGz(buffer, destDir) {
  const tmpFile = path.join(os.tmpdir(), "i18n-convert.tar.gz");
  fs.writeFileSync(tmpFile, buffer);
  execSync(`tar xzf "${tmpFile}" -C "${destDir}"`);
  fs.unlinkSync(tmpFile);
}

async function extractZip(buffer, destDir) {
  const tmpFile = path.join(os.tmpdir(), "i18n-convert.zip");
  fs.writeFileSync(tmpFile, buffer);
  execSync(
    `powershell -Command "Expand-Archive -Path '${tmpFile}' -DestinationPath '${destDir}' -Force"`
  );
  fs.unlinkSync(tmpFile);
}

async function main() {
  const target = getTarget();
  const url = getDownloadUrl(target);
  const isWindows = os.platform() === "win32";
  const binaryName = isWindows ? "i18n-convert.exe" : "i18n-convert";
  const binDir = path.join(__dirname, "bin");

  console.log(`Downloading i18n-convert v${VERSION} for ${target}...`);

  try {
    const buffer = await download(url);

    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    if (isWindows) {
      await extractZip(buffer, binDir);
    } else {
      await extractTarGz(buffer, binDir);
    }

    const binaryPath = path.join(binDir, binaryName);

    if (!fs.existsSync(binaryPath)) {
      throw new Error(`Binary not found after extraction: ${binaryPath}`);
    }

    if (!isWindows) {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log(`Successfully installed i18n-convert v${VERSION}`);
  } catch (error) {
    console.error(`Failed to install i18n-convert: ${error.message}`);
    console.error(
      "You can install manually from: https://github.com/i18n-agent/i18n-convert/releases"
    );
    process.exit(1);
  }
}

main();
