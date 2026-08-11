import assert from "node:assert/strict";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import {
  checkRepositoryVersionPolicy,
  validateVersionPolicy,
} from "./check-versions.mjs";

const repositoryRoot = fileURLToPath(new URL("..", import.meta.url));

test("the repository uses one application, Rust, and Node version policy", () => {
  assert.deepEqual(checkRepositoryVersionPolicy(repositoryRoot), []);
});

test("version validation reports application and toolchain mismatches", () => {
  const issues = validateVersionPolicy({
    applicationVersions: {
      packageJson: "0.1.0",
      cargoToml: "0.2.0",
      tauriConfig: "0.1.0",
    },
    rustVersions: {
      cargoMsrv: "1.85.0",
      toolchain: "1.97.1",
      workflows: ["1.97.1"],
    },
    nodeVersions: {
      engineMajor: 22,
      typesMajor: 24,
      versionFileMajor: 22,
      workflows: [22],
    },
    applicationIdentifier: "app.example.invalid",
  });

  assert.ok(issues.some((issue) => issue.includes("application version")));
  assert.ok(issues.some((issue) => issue.includes("Rust version")));
  assert.ok(issues.some((issue) => issue.includes("Node version")));
  assert.ok(issues.some((issue) => issue.includes("application identifier")));
});
