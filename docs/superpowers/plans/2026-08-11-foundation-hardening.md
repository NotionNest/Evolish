# Evolish Foundation Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the architecture, repository-governance, IPC-security, internationalization, build, and supply-chain gaps so Evolish can begin Phase 1 requirement implementation without foundational rework.

**Architecture:** Keep the existing modular monolith. Rust remains the owner of domain/application state and exposes explicitly permitted IPC DTOs; React consumes generated bindings and typed localized messages. CI proves frontend quality, Rust quality, and full Tauri integration on macOS, Windows, and Linux before protected `main` accepts a squash merge.

**Tech Stack:** Tauri 2, Rust 1.97.1, `serde`, `ts-rs` 12.0.1, React 19, TypeScript, native `Intl`, Vite, Vitest, GitHub Actions, Dependabot.

---

## File map

- `docs/architecture/*`, `docs/reference/pm/*`: accepted decisions and removal of stale open questions.
- `docs/roadmap/phase-1-delivery-plan.md`: dependency-ordered Phase 1 execution and evidence model.
- `src-tauri/src/application/bootstrap.rs`: internal application metadata only.
- `src-tauri/src/ipc/dto/*`: serialized IPC contracts and `ts-rs` export ownership.
- `src-tauri/src/ipc/bootstrap.rs`: Tauri command mapping application output to IPC output.
- `src/bridge/generated/*`: generated TypeScript declarations; never edited by hand.
- `src/bridge/appBootstrap.ts`: narrow typed invoke wrapper and rejection normalization.
- `src/i18n/*`: typed `zh-CN`/`en-US` catalogs, locale resolution, direction, and React provider.
- `.github/workflows/quality.yml`: frontend, Rust, binding drift, supply-chain, and three-platform Tauri build gates.
- `.github/dependabot.yml`, `SECURITY.md`, `LICENSE`: public repository governance.
- `scripts/check-versions.mjs`: tool/application version consistency.

### Task 1: Accept product and architecture decisions

**Files:**
- Modify: `docs/architecture/overall-architecture.md`
- Modify: `docs/architecture/technology-selection.md`
- Modify: `docs/reference/pm/phase-1-prd.md`
- Modify: `docs/reference/pm/product-decisions.md`
- Create: `docs/architecture/adr/0002-platform-delivery-strategy.md`
- Create: `docs/architecture/adr/0003-ipc-contract-and-command-security.md`
- Create: `docs/architecture/adr/0004-storage-and-provider-strategy.md`
- Create: `docs/roadmap/phase-1-delivery-plan.md`
- Create: `LICENSE`

- [x] Mark the PRD and overall architecture Accepted and replace already-decided open questions with a closed decision table.
- [x] Record SQLite/Rust ownership, compiled-in Phase 1 providers, macOS → Windows → Linux delivery order, X11 parity, Wayland capability matrix, public binary distribution, Apache-2.0, typed locales, and per-feature pain-point review.
- [x] Add ADRs with context, decision, consequences, rejected alternatives, and validation requirements.
- [x] Map every Phase 1 feature ID into ordered epics: foundation, input query, query supervision, providers/settings, windows/capture, OCR, dictionaries/TTS, integrations/hardening, release.
- [x] Define Issue evidence fields: requirement IDs, platform scope, acceptance commands, screenshots/fixtures, and known limitations.
- [x] Add the canonical Apache-2.0 license text.
- [x] Run `rg -n "Proposed|需求基线草案|当前待确认事项|待确认" docs` and verify only genuinely future decisions remain.
- [ ] Commit: `docs: accept foundation decisions and phase one roadmap`.

### Task 2: Make Rust the generated IPC contract source

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/application/bootstrap.rs`
- Delete: `src-tauri/src/domain/app_bootstrap.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/ipc/dto/mod.rs`
- Create: `src-tauri/src/ipc/dto/app_bootstrap.rs`
- Create: `src-tauri/src/ipc/dto/app_error.rs`
- Modify: `src-tauri/src/ipc/mod.rs`
- Modify: `src-tauri/src/ipc/bootstrap.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/build.rs`
- Modify: `src-tauri/capabilities/default.json`
- Create/generated: `src/bridge/generated/AppBootstrapDto.ts`
- Create/generated: `src/bridge/generated/AppErrorDto.ts`
- Modify: `src/bridge/appBootstrap.ts`

- [x] Add a failing Rust test that expects application metadata to map into a separate `AppBootstrapDto` and an export test that expects generated files under `src/bridge/generated`.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml` and verify failure because DTO/export code does not exist.
- [x] Add `ts-rs = { version = "12.0.1", features = ["serde-compat"] }`, derive `TS` only on IPC DTOs, and export during Rust tests.
- [x] Rename the command to `app_get_bootstrap` everywhere and import only generated TypeScript types in the frontend wrapper.
- [x] Configure `tauri_build::try_build` with `AppManifest::commands(&["app_get_bootstrap"])`; include the generated allow permission only in the main-window capability.
- [x] Assert the checked-in capability includes `allow-app-get-bootstrap`, excludes the generated deny permission, and grants the command to no window other than `main`.
- [x] Run Rust tests and verify generated bindings and mapping tests pass.
- [x] Run `pnpm typecheck` and `pnpm tauri build --debug --no-bundle` to verify permission generation and integration.
- [x] Add CI drift command: run binding exports, then `git diff --exit-code -- src/bridge/generated`.
- [ ] Commit: `feat(ipc): generate typed command contracts from Rust`.

### Task 3: Add typed internationalization before feature copy grows

**Files:**
- Create: `src/i18n/catalogs/en-US.ts`
- Create: `src/i18n/catalogs/zh-CN.ts`
- Create: `src/i18n/messages.ts`
- Create: `src/i18n/locale.ts`
- Create: `src/i18n/I18nProvider.tsx`
- Create: `src/i18n/locale.test.ts`
- Create: `src/i18n/I18nProvider.test.tsx`
- Modify: `src/main.tsx`
- Modify: `src/app/App.tsx`
- Modify: `src/app/App.test.tsx`

- [x] Write failing tests for exact locale resolution, Chinese language fallback, unsupported locale fallback to `en-US`, document `lang`/`dir`, translated startup states, and missing-key behavior.
- [x] Run focused tests and verify failures.
- [x] Implement a compile-time key-safe catalog API, native `Intl` locale primitives, `zh-CN` and `en-US` catalogs, and direction metadata prepared for RTL locales.
- [x] Replace every hardcoded user-facing App string with typed message keys.
- [x] Run focused tests and the full frontend suite.
- [ ] Commit: `feat(i18n): establish typed English and Chinese locales`.

### Task 4: Establish the typed error boundary

**Files:**
- Modify: `src-tauri/src/ipc/dto/app_error.rs`
- Create: `src/bridge/errors.ts`
- Modify: `src/bridge/appBootstrap.ts`
- Create: `src/bridge/errors.test.ts`
- Create: `src/app/AppErrorBoundary.tsx`
- Create: `src/app/AppErrorBoundary.test.tsx`
- Modify: `src/main.tsx`
- Modify: `src/app/App.tsx`

- [ ] Write failing tests for serialized Tauri string/object rejections, unknown rejection fallback, and render-error containment.
- [ ] Run focused Vitest tests and verify the expected failures.
- [ ] Implement `AppErrorDto` with stable code, message key, retryability, suggested action, source, and no raw cause.
- [ ] Implement `normalizeIpcError(unknown): AppErrorDto`; preserve safe string detail only as diagnostic detail, never as a UI classification.
- [ ] Add an application error boundary around the root application and keep user-facing copy localized.
- [ ] Run focused tests, then `pnpm test` and Rust tests.
- [ ] Commit: `feat(app): add typed IPC and render error boundaries`.

### Task 5: Make tool and application versions internally consistent

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `README.md`
- Create: `scripts/check-versions.mjs`
- Create: `scripts/check-versions.test.mjs`
- Modify: `package.json`
- Modify: `.github/workflows/quality.yml`

- [ ] Write a failing Node test for mismatched package/Cargo/Tauri versions and mismatched Rust/Node policy.
- [ ] Run the test and verify failure against the current Rust 1.85 / 1.97.1 split.
- [ ] Set Rust MSRV/toolchain/CI to 1.97.1 and Node policy/types/CI to Node 22.
- [ ] Add a version consistency check for `package.json`, `Cargo.toml`, and `tauri.conf.json` application versions.
- [ ] Add the check to `pnpm check`; run tests and build.
- [ ] Commit: `build: align application and toolchain versions`.

### Task 6: Prove full Tauri integration and supply-chain health in CI

**Files:**
- Modify: `.github/workflows/quality.yml`
- Create: `.github/dependabot.yml`
- Create: `SECURITY.md`
- Create: `deny.toml`
- Create: `scripts/check-js-licenses.mjs`
- Create: `scripts/check-js-licenses.test.mjs`
- Modify: `README.md`

- [ ] Pin every third-party Action to a full commit SHA with a version comment.
- [ ] Add checkout, Node/pnpm setup, and `pnpm install --frozen-lockfile` to every clean platform job before running `pnpm tauri build --debug --no-bundle` on macOS, Windows, and Ubuntu.
- [ ] Add `--locked` to Cargo commands and generated-binding drift verification.
- [ ] Add an independent job named `Supply chain` with checkout, Node/pnpm setup, frozen dependency installation, official-registry `pnpm audit`, `cargo deny check advisories licenses sources`, and the JavaScript production-license policy script.
- [ ] Write failing tests for allowed, disallowed, and unparseable JavaScript license reports before implementing `check-js-licenses.mjs`.
- [ ] Add `deny.toml` with an explicit SPDX allowlist and source policy; install the current pinned `cargo-deny` through a pinned installer Action.
- [ ] Configure monthly npm, Cargo, and GitHub Actions Dependabot updates with grouped non-major updates.
- [ ] Document supported versions, private vulnerability reporting, and response expectations in `SECURITY.md`.
- [ ] Commit: `ci: enforce integration and supply chain gates`.
- [ ] Enable GitHub dependency graph/Dependabot alerts and verify `gh api repos/NotionNest/Evolish/dependency-graph/sbom` returns the repository SBOM.

### Task 7: Remove verified initialization redundancy

**Files:**
- Modify: `package.json`
- Modify: `.github/workflows/quality.yml`
- Modify: `src/app/app.css`
- Modify: `src/app/App.tsx`
- Modify/generated: `src/bridge/generated/AppBootstrapDto.ts`

- [ ] Add or adjust tests so the application renders its Rust-provided product name.
- [ ] Remove `vite preview`, stale `master` trigger, and the no-op reduced-motion rule.
- [ ] Scope the global `h1` selector to the startup panel.
- [ ] Run frontend tests, lint, typecheck, and build.
- [ ] Commit: `refactor: remove initialization-only redundancy`.

### Task 8: Validate the complete foundation

**Files:**
- Modify if required by failures only.

- [ ] Run `pnpm install --frozen-lockfile`.
- [ ] Run `pnpm check`.
- [ ] Run `cargo fmt --manifest-path src-tauri/Cargo.toml --check`.
- [ ] Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --locked`.
- [ ] Run `pnpm tauri build --debug --no-bundle`.
- [ ] Run `pnpm audit --audit-level low --registry=https://registry.npmjs.org`.
- [ ] Run `git diff --check`, `git grep -nIE '(AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{36,}|-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----)' -- . ':!pnpm-lock.yaml'`, and confirm a clean intended diff.
- [ ] Commit only evidence/document corrections if required.

### Task 9: Publish through the new workflow and protect main

**Files/remote state:**
- GitHub Pull Request from `agent/foundation-hardening` to `main`
- GitHub branch protection/ruleset for `main`
- GitHub security settings

- [ ] Push `agent/foundation-hardening` and open a draft PR with Conventional Commit title and full validation evidence.
- [ ] Verify the checks named `Frontend checks`, `Rust checks (macos-latest)`, `Rust checks (windows-latest)`, `Rust checks (ubuntu-24.04)`, and `Supply chain` pass without warnings; fix failures through tests and additional conventional commits.
- [ ] After the PR establishes exact check contexts, configure `main` to require PRs, `Commit conventions`, `Frontend checks`, all three named Rust checks, and `Supply chain`; also require linear history and resolved conversations, forbid force pushes/deletion, and require zero approvals for the solo-maintainer phase.
- [ ] Mark the protected PR ready and squash merge using the PR title only after every required check passes.
- [ ] Enable Dependabot alerts/security updates and verify Secret Scanning/Push Protection remain enabled.
- [ ] Verify local `main`, `origin/main`, repository license, default branch, merge policy, and latest CI are all consistent.
