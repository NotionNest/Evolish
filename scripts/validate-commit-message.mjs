import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const HEADER_LIMIT = 100;
const CONVENTIONAL_HEADER =
  /^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9][a-z0-9._/-]*\))?!?: \S.*$/;
const GENERATED_COMMIT = /^(Merge |Revert ")/;

export function validateCommitMessage(message) {
  const header = message.split(/\r?\n/, 1)[0]?.trim() ?? "";
  const errors = [];

  if (header.length === 0) {
    errors.push("commit message header cannot be empty");
  } else if (!GENERATED_COMMIT.test(header) && !CONVENTIONAL_HEADER.test(header)) {
    errors.push(
      "header must match <type>(<optional-scope>): <description> using an allowed type",
    );
  }

  if (header.length > HEADER_LIMIT) {
    errors.push(`header must not exceed ${HEADER_LIMIT} characters`);
  }

  return errors;
}

function readMessage(args) {
  if (args[0] === "--file" && args[1]) {
    return readFileSync(args[1], "utf8");
  }

  if (args[0] === "--env" && args[1]) {
    return process.env[args[1]] ?? "";
  }

  return args.join(" ");
}

function runCli() {
  const errors = validateCommitMessage(readMessage(process.argv.slice(2)));

  if (errors.length === 0) {
    return;
  }

  process.stderr.write("Invalid Conventional Commit message:\n");
  for (const error of errors) {
    process.stderr.write(`- ${error}\n`);
  }
  process.stderr.write("Example: feat(query): add selection capture pipeline\n");
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  runCli();
}
