import { spawnSync } from "node:child_process";

function configure(key, value) {
  const result = spawnSync("git", ["config", "--local", key, value], {
    encoding: "utf8",
  });

  if (result.status !== 0) {
    const detail = result.stderr.trim() || `git config exited with ${result.status}`;
    throw new Error(`Unable to configure ${key}: ${detail}`);
  }
}

const repositoryCheck = spawnSync("git", ["rev-parse", "--git-dir"], {
  encoding: "utf8",
});

if (repositoryCheck.status === 0) {
  configure("core.hooksPath", ".githooks");
  configure("commit.template", ".gitmessage");
  process.stdout.write("Git hooks and commit template configured.\n");
} else {
  process.stdout.write("Git repository not detected; hook setup skipped.\n");
}
