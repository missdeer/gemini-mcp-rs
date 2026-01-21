#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const https = require("https");
const os = require("os");
const crypto = require("crypto");

const PACKAGE_NAME = "gemini-mcp-rs";
const REPO_OWNER = "missdeer";
const REPO_NAME = "gemini-mcp-rs";

const MAX_REDIRECTS = 10;
const REQUEST_TIMEOUT = 60000; // 60 seconds
const MAX_RETRIES = 3;
const RETRY_DELAY = 1000; // 1 second

// Get proxy agent if proxy is configured
function getProxyAgent() {
  const proxyUrl =
    process.env.HTTPS_PROXY ||
    process.env.https_proxy ||
    process.env.HTTP_PROXY ||
    process.env.http_proxy;
  if (!proxyUrl) return null;

  try {
    const { HttpsProxyAgent } = require("https-proxy-agent");
    return new HttpsProxyAgent(proxyUrl);
  } catch {
    // https-proxy-agent not available, skip proxy
    return null;
  }
}

// Helper for retry with exponential backoff
async function withRetry(fn, retries = MAX_RETRIES) {
  let lastError;
  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      return await fn();
    } catch (err) {
      lastError = err;
      // Don't retry on non-retryable errors
      if (
        err.message.includes("Unsupported") ||
        err.message.includes("rate limit") ||
        err.message.includes("404")
      ) {
        throw err;
      }
      if (attempt < retries - 1) {
        const delay = RETRY_DELAY * Math.pow(2, attempt);
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
  }
  throw lastError;
}

// Get package version from package.json
function getPackageVersion() {
  return require("./package.json").version;
}

// Get cache directory based on OS
function getCacheDir() {
  const homeDir = os.homedir();
  const version = getPackageVersion();
  let baseDir;

  switch (process.platform) {
    case "win32":
      baseDir = path.join(
        process.env.LOCALAPPDATA || path.join(homeDir, "AppData", "Local"),
        PACKAGE_NAME
      );
      break;
    case "darwin":
      baseDir = path.join(homeDir, "Library", "Caches", PACKAGE_NAME);
      break;
    default:
      baseDir = path.join(
        process.env.XDG_CACHE_HOME || path.join(homeDir, ".cache"),
        PACKAGE_NAME
      );
  }

  // Include version in cache path to handle upgrades
  return path.join(baseDir, version);
}

// Get asset name based on platform (matching release.yml)
function getAssetName() {
  const platform = process.platform;
  const arch = process.arch;

  switch (platform) {
    case "darwin":
      // macOS uses universal binary (supports both x64 and arm64)
      return "gemini-mcp-rs_Darwin_universal.tar.gz";
    case "linux":
      if (arch === "x64") {
        return "gemini-mcp-rs_Linux_x86_64.tar.gz";
      } else if (arch === "arm64") {
        return "gemini-mcp-rs_Linux_arm64.tar.gz";
      }
      throw new Error(
        `Unsupported architecture: ${arch} on Linux. Only x64 and arm64 are supported.`
      );
    case "win32":
      if (arch === "x64") {
        return "gemini-mcp-rs_Windows_x86_64.zip";
      } else if (arch === "arm64") {
        return "gemini-mcp-rs_Windows_arm64.zip";
      }
      throw new Error(
        `Unsupported architecture: ${arch} on Windows. Only x64 and arm64 are supported.`
      );
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
}

function getBinaryName() {
  return process.platform === "win32" ? `${PACKAGE_NAME}.exe` : PACKAGE_NAME;
}

function getBinaryPath() {
  const cacheDir = getCacheDir();
  const binaryName = getBinaryName();
  return path.join(cacheDir, binaryName);
}

function httpsGet(url, options = {}, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    if (redirectCount > MAX_REDIRECTS) {
      reject(new Error("Too many redirects"));
      return;
    }
    const agent = getProxyAgent();
    const req = https.get(url, { ...options, ...(agent && { agent }) }, (res) => {
      // Handle redirects
      if (
        res.statusCode >= 300 &&
        res.statusCode < 400 &&
        res.headers.location
      ) {
        // Consume response to free up connection
        res.resume();
        const redirectUrl = res.headers.location;
        // Only follow HTTPS redirects
        if (!redirectUrl.startsWith("https://")) {
          reject(new Error(`Insecure redirect to: ${redirectUrl}`));
          return;
        }
        httpsGet(redirectUrl, options, redirectCount + 1)
          .then(resolve)
          .catch(reject);
        return;
      }

      if (res.statusCode === 403) {
        res.resume();
        reject(
          new Error(
            "GitHub API rate limit exceeded. Please try again later or set GITHUB_TOKEN environment variable."
          )
        );
        return;
      }

      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
        return;
      }

      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    });

    req.on("error", reject);
    req.setTimeout(REQUEST_TIMEOUT, () => {
      req.destroy();
      reject(new Error("Request timeout"));
    });
  });
}

// Parse JSON with helpful error message
function parseJSON(data, context) {
  try {
    return JSON.parse(data.toString());
  } catch (err) {
    const preview = data.toString().slice(0, 200);
    throw new Error(
      `Failed to parse ${context} response. GitHub may be experiencing issues. Response preview: ${preview}`
    );
  }
}

async function getReleaseByTag(version) {
  const tag = `v${version}`;
  const url = `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/tags/${tag}`;
  const options = {
    headers: {
      "User-Agent": PACKAGE_NAME,
      Accept: "application/vnd.github.v3+json",
      ...(process.env.GITHUB_TOKEN && {
        Authorization: `token ${process.env.GITHUB_TOKEN}`,
      }),
    },
  };

  try {
    const data = await httpsGet(url, options);
    return parseJSON(data, "release");
  } catch (error) {
    // If the specific version tag doesn't exist, fall back to latest
    if (error.message.includes("404")) {
      console.error(
        `Release v${version} not found, falling back to latest release...`
      );
      return getLatestRelease();
    }
    throw error;
  }
}

async function getLatestRelease() {
  const url = `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`;
  const options = {
    headers: {
      "User-Agent": PACKAGE_NAME,
      Accept: "application/vnd.github.v3+json",
      ...(process.env.GITHUB_TOKEN && {
        Authorization: `token ${process.env.GITHUB_TOKEN}`,
      }),
    },
  };

  const data = await httpsGet(url, options);
  return parseJSON(data, "latest release");
}

function downloadToFile(url, destPath, options = {}, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    if (redirectCount > MAX_REDIRECTS) {
      reject(new Error("Too many redirects"));
      return;
    }

    const file = fs.createWriteStream(destPath);
    const agent = getProxyAgent();
    const req = https.get(url, { ...options, ...(agent && { agent }) }, (res) => {
      // Handle redirects
      if (
        res.statusCode >= 300 &&
        res.statusCode < 400 &&
        res.headers.location
      ) {
        res.resume(); // Consume response
        file.close(() => {
          try {
            fs.unlinkSync(destPath);
          } catch {}
          const redirectUrl = res.headers.location;
          if (!redirectUrl.startsWith("https://")) {
            reject(new Error(`Insecure redirect to: ${redirectUrl}`));
            return;
          }
          downloadToFile(redirectUrl, destPath, options, redirectCount + 1)
            .then(resolve)
            .catch(reject);
        });
        return;
      }

      if (res.statusCode !== 200) {
        res.resume(); // Consume response
        file.close(() => {
          try {
            fs.unlinkSync(destPath);
          } catch {}
          reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
        });
        return;
      }

      res.pipe(file);
      file.on("finish", () => {
        file.close(() => resolve());
      });
      file.on("error", (err) => {
        file.close(() => {
          try {
            fs.unlinkSync(destPath);
          } catch {}
          reject(err);
        });
      });
    });

    req.on("error", (err) => {
      file.close(() => {
        try {
          fs.unlinkSync(destPath);
        } catch {}
        reject(err);
      });
    });

    req.setTimeout(REQUEST_TIMEOUT, () => {
      req.destroy();
      file.close(() => {
        try {
          fs.unlinkSync(destPath);
        } catch {}
        reject(new Error("Download timeout"));
      });
    });
  });
}

async function extractTarGz(archivePath, destDir) {
  return new Promise((resolve, reject) => {
    // Redirect extraction output to stderr to keep stdout clean for MCP JSON-RPC
    const tar = spawn("tar", ["-xzf", archivePath, "-C", destDir], {
      stdio: ["ignore", "inherit", "inherit"],
    });
    tar.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`tar exited with code ${code}`));
    });
    tar.on("error", (err) => {
      if (err.code === "ENOENT") {
        reject(new Error("tar command not found. Please install tar."));
      } else {
        reject(err);
      }
    });
  });
}

async function extractZip(archivePath, destDir) {
  return new Promise((resolve, reject) => {
    // Escape paths for PowerShell: escape backticks and single quotes
    const escapePath = (p) => p.replace(/`/g, "``").replace(/'/g, "''");
    // Redirect extraction output to stderr to keep stdout clean for MCP JSON-RPC
    const unzipProcess = spawn(
      "powershell",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        `Expand-Archive -LiteralPath '${escapePath(archivePath)}' -DestinationPath '${escapePath(destDir)}' -Force`,
      ],
      { stdio: ["ignore", "inherit", "inherit"] }
    );
    unzipProcess.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`PowerShell Expand-Archive exited with code ${code}`));
    });
    unzipProcess.on("error", (err) => {
      if (err.code === "ENOENT") {
        reject(new Error("PowerShell not found. Please install PowerShell 5.0+."));
      } else {
        reject(err);
      }
    });
  });
}

// Move file with fallback for cross-device moves
function moveFile(src, dest) {
  try {
    fs.renameSync(src, dest);
  } catch (err) {
    if (err.code === "EXDEV") {
      // Cross-device move: copy + delete
      fs.copyFileSync(src, dest);
      fs.unlinkSync(src);
    } else {
      throw err;
    }
  }
}

// Create a lock file to prevent concurrent downloads
function acquireLock(lockPath) {
  try {
    fs.writeFileSync(lockPath, process.pid.toString(), { flag: "wx" });
    return true;
  } catch (err) {
    if (err.code === "EEXIST") {
      // Check if the process that created the lock is still running
      try {
        const pid = parseInt(fs.readFileSync(lockPath, "utf8"), 10);
        try {
          process.kill(pid, 0); // Check if process exists
          return false; // Process is still running
        } catch {
          // Process is not running, remove stale lock
          fs.unlinkSync(lockPath);
          return acquireLock(lockPath);
        }
      } catch {
        return false;
      }
    }
    throw err;
  }
}

function releaseLock(lockPath) {
  try {
    fs.unlinkSync(lockPath);
  } catch {
    // Ignore errors
  }
}

async function downloadAndExtract(cacheDir) {
  const assetName = getAssetName();
  const binaryName = getBinaryName();
  const binaryPath = path.join(cacheDir, binaryName);
  const lockPath = path.join(cacheDir, ".lock");
  const tempId = crypto.randomBytes(8).toString("hex");

  // Ensure cache directory exists
  fs.mkdirSync(cacheDir, { recursive: true });

  // Try to acquire lock
  if (!acquireLock(lockPath)) {
    // Wait for other process to complete
    console.error("Another process is downloading, waiting...");
    let attempts = 0;
    while (!fs.existsSync(binaryPath) && attempts < 60) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      attempts++;
    }
    if (fs.existsSync(binaryPath)) {
      return binaryPath;
    }
    throw new Error("Timeout waiting for download to complete");
  }

  try {
    // Double check after acquiring lock
    if (fs.existsSync(binaryPath)) {
      return binaryPath;
    }

    // Get release for the specific version (with retry)
    const version = getPackageVersion();
    console.error(`Downloading ${PACKAGE_NAME} v${version}...`);

    const release = await withRetry(() => getReleaseByTag(version));
    const asset = release.assets.find((a) => a.name === assetName);
    const checksumsAsset = release.assets.find(
      (a) => a.name === "sha256sum.json"
    );

    if (!asset) {
      const availableAssets = release.assets.map((a) => a.name).join(", ");
      throw new Error(
        `No matching asset found: ${assetName}. Available: ${availableAssets}`
      );
    }
    if (!checksumsAsset) {
      throw new Error("sha256sum.json not found in release assets.");
    }

    const downloadOptions = {
      headers: {
        "User-Agent": PACKAGE_NAME,
        Accept: "application/octet-stream",
        ...(process.env.GITHUB_TOKEN && {
          Authorization: `token ${process.env.GITHUB_TOKEN}`,
        }),
      },
    };

    // Download checksums
    const checksumsData = await httpsGet(
      checksumsAsset.browser_download_url,
      downloadOptions
    );
    const checksums = parseJSON(checksumsData, "checksums");
    const expectedChecksum = checksums[assetName];
    if (!expectedChecksum) {
      throw new Error(`Checksum not found for ${assetName} in sha256sum.json`);
    }

    // Download to temporary file first
    const tempArchive = path.join(cacheDir, `${tempId}-${assetName}`);
    const tempExtractDir = path.join(cacheDir, `${tempId}-extract`);

    await withRetry(() =>
      downloadToFile(asset.browser_download_url, tempArchive, downloadOptions)
    );

    // Verify checksum
    const hash = crypto.createHash("sha256");
    const stream = fs.createReadStream(tempArchive);
    stream.on("data", (d) => hash.update(d));
    await new Promise((resolve, reject) => {
      stream.on("end", resolve);
      stream.on("error", reject);
    });
    const actualChecksum = hash.digest("hex");

    if (actualChecksum !== expectedChecksum) {
      fs.unlinkSync(tempArchive);
      throw new Error(
        `Checksum mismatch for ${assetName}. Expected: ${expectedChecksum}, got: ${actualChecksum}`
      );
    }

    // Extract to temporary directory
    console.error("Extracting...");
    fs.mkdirSync(tempExtractDir, { recursive: true });

    if (assetName.endsWith(".zip")) {
      await extractZip(tempArchive, tempExtractDir);
    } else {
      await extractTarGz(tempArchive, tempExtractDir);
    }

    // Find the binary in the extracted directory
    const extractedBinary = path.join(tempExtractDir, binaryName);
    if (!fs.existsSync(extractedBinary)) {
      throw new Error(
        `Binary not found in archive. Expected: ${binaryName} in extracted contents.`
      );
    }

    // Atomic move to final location (with cross-device fallback)
    moveFile(extractedBinary, binaryPath);

    // Make binary executable on Unix
    if (process.platform !== "win32") {
      fs.chmodSync(binaryPath, 0o755);
    }

    // Clean up
    fs.unlinkSync(tempArchive);
    fs.rmSync(tempExtractDir, { recursive: true, force: true });

    console.error(`Installed ${PACKAGE_NAME} to ${binaryPath}`);
    return binaryPath;
  } finally {
    releaseLock(lockPath);
  }
}

async function ensureBinary() {
  const binaryPath = getBinaryPath();

  if (!fs.existsSync(binaryPath)) {
    try {
      await downloadAndExtract(getCacheDir());
    } catch (error) {
      console.error(`Failed to download ${PACKAGE_NAME}: ${error.message}`);
      console.error("");
      console.error("You can install manually:");
      console.error(
        "  1. Download from https://github.com/missdeer/gemini-mcp-rs/releases"
      );
      console.error(`  2. Place binary at: ${binaryPath}`);
      console.error("");
      console.error("Or install via cargo:");
      console.error("  cargo install gemini-mcp-rs");
      process.exit(1);
    }
  }

  return binaryPath;
}

async function run() {
  const binaryPath = await ensureBinary();

  // Run the binary with all arguments
  const args = process.argv.slice(2);
  const child = spawn(binaryPath, args, {
    stdio: "inherit",
    env: process.env,
  });

  // Forward signals to child process
  const signals = ["SIGINT", "SIGTERM", "SIGHUP"];
  signals.forEach((signal) => {
    process.on(signal, () => {
      if (!child.killed) {
        child.kill(signal);
      }
    });
  });

  child.on("error", (error) => {
    console.error(`Failed to start ${PACKAGE_NAME}: ${error.message}`);
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.exit(128 + (os.constants.signals[signal] || 0));
    }
    process.exit(code ?? 0);
  });
}

run();
