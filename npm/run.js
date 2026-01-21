#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");

const PACKAGE_NAME = "gemini-mcp-rs";

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

function getBinaryName() {
  return process.platform === "win32" ? `${PACKAGE_NAME}.exe` : PACKAGE_NAME;
}

function getBinaryPath() {
    const cacheDir = getCacheDir();
    const binaryName = getBinaryName();
    return path.join(cacheDir, binaryName);
}

function run() {
  const binaryPath = getBinaryPath();

  // Check if binary exists
  if (!fs.existsSync(binaryPath)) {
    console.error(`Binary not found at ${binaryPath}`);
    console.error(`Please run "npm install" or "yarn install" to download the binary.`);
    process.exit(1);
  }

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