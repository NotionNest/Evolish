import assert from "node:assert/strict";
import { test } from "node:test";

import { validateLicenseReport } from "./check-js-licenses.mjs";

const dependency = {
  name: "example-package",
  versions: ["1.0.0"],
  paths: ["node_modules/example-package"],
  license: "MIT",
};

test("allows approved SPDX licenses and expressions", () => {
  assert.deepEqual(
    validateLicenseReport({
      MIT: [dependency],
      "Apache-2.0 OR MIT": [
        { ...dependency, name: "dual-licensed-package" },
      ],
    }),
    [],
  );
});

test("rejects dependencies with disallowed licenses", () => {
  const issues = validateLicenseReport({
    "GPL-3.0-only": [{ ...dependency, license: "GPL-3.0-only" }],
  });

  assert.ok(issues.some((issue) => issue.includes("GPL-3.0-only")));
});

test("rejects malformed or unparseable license reports", () => {
  assert.ok(validateLicenseReport(null).length > 0);
  assert.ok(validateLicenseReport({ "SEE LICENSE IN LICENSE": [] }).length > 0);
});
