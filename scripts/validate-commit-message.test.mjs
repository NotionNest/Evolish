import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { validateCommitMessage } from "./validate-commit-message.mjs";

describe("validateCommitMessage", () => {
  it("accepts Conventional Commit headers", () => {
    const validMessages = [
      "feat(query): add selection capture pipeline",
      "fix!: remove incompatible provider protocol",
      "docs: define contribution workflow",
      "chore(deps/rust): update Tauri",
    ];

    for (const message of validMessages) {
      assert.deepEqual(validateCommitMessage(message), []);
    }
  });

  it("accepts Git-generated merge and revert commits", () => {
    assert.deepEqual(validateCommitMessage("Merge branch 'feature/query'"), []);
    assert.deepEqual(validateCommitMessage('Revert "feat: add query flow"'), []);
  });

  it("rejects unknown types and missing descriptions", () => {
    assert.notDeepEqual(validateCommitMessage("feature: add query flow"), []);
    assert.notDeepEqual(validateCommitMessage("feat(query):"), []);
  });

  it("rejects headers longer than 100 characters", () => {
    const message = `feat(query): ${"a".repeat(90)}`;

    assert.match(validateCommitMessage(message).join(" "), /100 characters/);
  });
});
