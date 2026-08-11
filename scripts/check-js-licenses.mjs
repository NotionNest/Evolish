import { execFileSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const allowedLicenses = new Set([
  "0BSD",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "CC0-1.0",
  "ISC",
  "MIT",
  "MPL-2.0",
  "Unicode-3.0",
  "Zlib",
]);

const allowedExceptions = new Set(["LLVM-exception"]);
const operators = new Set(["AND", "OR"]);

function tokenizeSpdxExpression(expression) {
  if (!/^[A-Za-z0-9.+()\s-]+$/u.test(expression)) {
    return null;
  }

  const tokens = expression.match(/\(|\)|[A-Za-z0-9][A-Za-z0-9.+-]*/gu);
  if (!tokens) {
    return null;
  }

  const compactExpression = expression.replaceAll(/\s/gu, "");
  if (tokens.join("") !== compactExpression) {
    return null;
  }

  return tokens;
}

function validateSpdxExpression(expression) {
  const tokens = tokenizeSpdxExpression(expression);
  if (!tokens) {
    return false;
  }

  let depth = 0;
  let state = "start";

  for (const token of tokens) {
    if (token === "(") {
      if (!["start", "operator", "open"].includes(state)) {
        return false;
      }
      depth += 1;
      state = "open";
      continue;
    }

    if (token === ")") {
      if (depth === 0 || !["license", "exception", "close"].includes(state)) {
        return false;
      }
      depth -= 1;
      state = "close";
      continue;
    }

    if (operators.has(token)) {
      if (!["license", "exception", "close"].includes(state)) {
        return false;
      }
      state = "operator";
      continue;
    }

    if (token === "WITH") {
      if (state !== "license") {
        return false;
      }
      state = "with";
      continue;
    }

    if (state === "with") {
      if (!allowedExceptions.has(token)) {
        return false;
      }
      state = "exception";
      continue;
    }

    if (!["start", "operator", "open"].includes(state)) {
      return false;
    }
    if (!allowedLicenses.has(token)) {
      return false;
    }
    state = "license";
  }

  return (
    depth === 0 && ["license", "exception", "close"].includes(state)
  );
}

export function validateLicenseReport(report) {
  if (
    typeof report !== "object" ||
    report === null ||
    Array.isArray(report)
  ) {
    return ["JavaScript license report must be a JSON object"];
  }

  const entries = Object.entries(report);
  if (entries.length === 0) {
    return ["JavaScript license report must contain production dependencies"];
  }

  const issues = [];
  for (const [expression, dependencies] of entries) {
    if (!validateSpdxExpression(expression)) {
      issues.push(`disallowed or invalid SPDX expression: ${expression}`);
    }

    if (!Array.isArray(dependencies) || dependencies.length === 0) {
      issues.push(`license group has no dependency evidence: ${expression}`);
      continue;
    }

    for (const dependency of dependencies) {
      if (
        typeof dependency !== "object" ||
        dependency === null ||
        typeof dependency.name !== "string" ||
        !Array.isArray(dependency.versions) ||
        dependency.versions.length === 0
      ) {
        issues.push(`malformed dependency evidence under: ${expression}`);
      }
    }
  }

  return issues;
}

function run() {
  const reportJson = execFileSync(
    "pnpm",
    ["licenses", "list", "--prod", "--json"],
    { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );

  let report;
  try {
    report = JSON.parse(reportJson);
  } catch {
    console.error("Unable to parse pnpm production license report as JSON");
    process.exitCode = 1;
    return;
  }

  const issues = validateLicenseReport(report);
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
