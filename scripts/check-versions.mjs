import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const expectedRustVersion = "1.97.1";
const expectedNodeMajor = 22;
const expectedApplicationIdentifier = "io.github.notionnest.evolish";

function readText(rootDirectory, relativePath) {
  return readFileSync(path.join(rootDirectory, relativePath), "utf8");
}

function requiredMatch(text, pattern, label) {
  const value = text.match(pattern)?.[1];
  if (!value) {
    throw new Error(`Unable to read ${label}`);
  }
  return value;
}

function parseMajor(versionRange, label) {
  const major = versionRange.match(/\d+/u)?.[0];
  if (!major) {
    throw new Error(`Unable to read ${label} major version`);
  }
  return Number.parseInt(major, 10);
}

export function validateVersionPolicy(policy) {
  const issues = [];

  const applicationVersions = Object.values(policy.applicationVersions);
  if (new Set(applicationVersions).size !== 1) {
    issues.push(
      `application version mismatch: ${applicationVersions.join(", ")}`,
    );
  }

  const rustVersions = [
    policy.rustVersions.cargoMsrv,
    policy.rustVersions.toolchain,
    ...policy.rustVersions.workflows,
  ];
  if (
    rustVersions.some((version) => version !== expectedRustVersion) ||
    policy.rustVersions.workflows.length === 0
  ) {
    issues.push(
      `Rust version policy must be ${expectedRustVersion}: ${rustVersions.join(", ")}`,
    );
  }

  const nodeVersions = [
    policy.nodeVersions.engineMajor,
    policy.nodeVersions.typesMajor,
    policy.nodeVersions.versionFileMajor,
    ...policy.nodeVersions.workflows,
  ];
  if (
    nodeVersions.some((version) => version !== expectedNodeMajor) ||
    policy.nodeVersions.workflows.length === 0
  ) {
    issues.push(
      `Node version policy must use major ${expectedNodeMajor}: ${nodeVersions.join(", ")}`,
    );
  }

  if (policy.applicationIdentifier !== expectedApplicationIdentifier) {
    issues.push(
      `application identifier must be ${expectedApplicationIdentifier}: ${policy.applicationIdentifier}`,
    );
  }

  return issues;
}

export function checkRepositoryVersionPolicy(rootDirectory) {
  const packageJson = JSON.parse(readText(rootDirectory, "package.json"));
  const cargoToml = readText(rootDirectory, "src-tauri/Cargo.toml");
  const tauriConfig = JSON.parse(
    readText(rootDirectory, "src-tauri/tauri.conf.json"),
  );
  const rustToolchain = readText(rootDirectory, "rust-toolchain.toml");
  const qualityWorkflow = readText(
    rootDirectory,
    ".github/workflows/quality.yml",
  );
  const nodeVersionPath = path.join(rootDirectory, ".node-version");

  const workflowNodeVersions = [
    ...qualityWorkflow.matchAll(/node-version:\s*["']?(\d+)/gu),
  ].map((match) => Number.parseInt(match[1], 10));
  const workflowRustVersions = [
    ...qualityWorkflow.matchAll(/toolchain:\s*["']?([\d.]+)/gu),
  ].map((match) => match[1]);

  return validateVersionPolicy({
    applicationVersions: {
      packageJson: packageJson.version,
      cargoToml: requiredMatch(
        cargoToml,
        /^version\s*=\s*"([^"]+)"/mu,
        "Cargo package version",
      ),
      tauriConfig: tauriConfig.version,
    },
    rustVersions: {
      cargoMsrv: requiredMatch(
        cargoToml,
        /^rust-version\s*=\s*"([^"]+)"/mu,
        "Cargo rust-version",
      ),
      toolchain: requiredMatch(
        rustToolchain,
        /^channel\s*=\s*"([^"]+)"/mu,
        "Rust toolchain channel",
      ),
      workflows: workflowRustVersions,
    },
    nodeVersions: {
      engineMajor: parseMajor(packageJson.engines.node, "Node engine"),
      typesMajor: parseMajor(
        packageJson.devDependencies["@types/node"],
        "@types/node",
      ),
      versionFileMajor: existsSync(nodeVersionPath)
        ? parseMajor(readFileSync(nodeVersionPath, "utf8"), ".node-version")
        : null,
      workflows: workflowNodeVersions,
    },
    applicationIdentifier: tauriConfig.identifier,
  });
}

function run() {
  const issues = checkRepositoryVersionPolicy(process.cwd());
  if (issues.length > 0) {
    for (const issue of issues) {
      console.error(`- ${issue}`);
    }
    process.exitCode = 1;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  run();
}
