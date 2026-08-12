# Text Input Translation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the complete accepted text-input translation workflow: explicit submission, local language direction, AI-only primary/on-demand providers, validated complete results, local history, favorites, privacy, and system TTS.

**Architecture:** A Rust `AppKernel` owns domain rules, query supervision, AI adapters, SQLite, credentials, and speech. React renders feature-local reducer state and communicates through generated typed Tauri commands and Channels; only validated complete result DTOs cross IPC. Implementation follows the accepted PRD and technical design with TDD and independent commits at every boundary.

**Tech Stack:** Tauri 2, Rust 2024, Tokio, SQLx/SQLite, Reqwest, platform credential store, Lingua, React 19, TypeScript, Vitest/React Testing Library, `serde`, `ts-rs`.

**Specifications:** `docs/reference/pm/text-input-translation-prd.md`, `docs/architecture/text-input-translation-technical-design.md`, `docs/architecture/adr/0005-ai-only-translation-and-on-demand-query.md`, `docs/architecture/adr/0006-two-window-product-model.md`.

---

## Acceptance gate

Passed on 2026-08-12. The product owner accepted `docs/architecture/text-input-translation-technical-design.md` and all six implementation decisions recorded in its Section 20. This plan is approved for sequential execution from Task 1.

## Delivery rules

- Work in a dedicated feature worktree created from current `main` after the accepted documentation change is merged.
- Before changing any dependency/API integration, use Context7 for current official documentation as required by `AGENTS.md`.
- For every behavior: write the focused failing test, run and confirm the expected failure, implement the behavior, then run focused and affected suites.
- Do not use live paid AI calls in CI; use local mock HTTP servers and sanitized fixtures.
- Never implement automatic provider fallback, partial正文 rendering, plaintext secret persistence, or frontend SQLite access.
- Do not create a Floating/side result window or separate Settings WebView. This plan grants translation-management permissions only to Main; the later Mini implementation receives a separate minimum capability.
- Every task ends with generated-binding drift checks where applicable and a Conventional Commit.

## Dependency additions

Resolve exact compatible versions from current official documentation during implementation and commit them through lockfiles:

- Runtime/domain: `async-trait`, `tokio`, `tokio-util`, `uuid`, `time`, `url`, `unicode-segmentation`, `secrecy`, `zeroize`.
- Infrastructure: `sqlx` with SQLite/migrations/runtime features, `reqwest` with Rustls/JSON, accepted platform credential implementation, `lingua` with only Phase 1 language features.
- Testing: in-process HTTP mock server and `tempfile`.
- UI/settings: Radix primitives and React Hook Form when their tasks begin; Tauri 2 official clipboard-manager plugin is initialized in Rust and wrapped by a constrained custom command.

## Task sequence

### Task 1: Lock toolchain dependencies and module boundaries

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/lib.rs`
- Create: `src-tauri/src/domain/mod.rs`, `src-tauri/src/application/translation/mod.rs`, `src-tauri/src/infrastructure/mod.rs`
- Test: `src-tauri/src/lib.rs`

- [x] Write a failing architecture test proving `domain` is independent of Tauri, SQLx and Reqwest, while application ports remain importable without a runtime.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml architecture_boundary --locked`; expect failure because modules/dependencies do not exist.
- [x] Use Context7 to verify current feature flags and APIs, then add the production/test dependencies and focused module roots without relaxing `unsafe_code` or Clippy policy.
- [x] Run the focused test, full Clippy and `cargo deny`; expect all to pass with no unreviewed license/source exception.
- [x] Commit: `build: add translation core dependencies`.

### Task 2: Define identifiers, language catalog and query intent

**Requirements:** `TIT-LANG-01`–`TIT-LANG-07`.

**Files:**
- Create: `src-tauri/src/domain/language.rs`, `src-tauri/src/domain/query_intent.rs`, `src-tauri/src/domain/translation.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Test: adjacent Rust test modules

- [x] Write failing tests for UUID v7 generation/parse, unique BCP 47 tags for all 48 languages, writing direction, provider-code mapping failure and stable serialization.
- [x] Write failing intent fixtures for Chinese/English words, phrases, sentences, paragraphs, mixed Unicode, code and manual override.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml domain:: --locked`; expect missing types/rules.
- [x] Implement closed enums and validated value objects: session/attempt/profile/mode IDs, result version, language selection/direction and query intent.
- [x] Rerun domain tests and Clippy; expect deterministic fixtures and explicit invalid-value errors.
- [x] Commit: `feat(domain): define translation language and intent models`.

### Task 3: Define adaptive results and attempt state machine

**Requirements:** `TIT-RES-01`–`TIT-RES-08`, `TIT-RETRY-03`–`TIT-RETRY-05`.

**Files:**
- Modify: `src-tauri/src/domain/translation.rs`
- Create: `src-tauri/src/domain/error.rs`, `src-tauri/src/application/translation/result_validator.rs`
- Test: adjacent Rust test modules

- [x] Write failing schema tests for valid Word/Phrase/Sentence/LongText V1 payloads and invalid empty translation, missing/duplicate paragraph indexes, intent mismatch and unknown values.
- [x] Write failing transition tests for `NotRequested → Queued → Running → Succeeded/Failed/Cancelled`; retry must create a new attempt ID/version.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml translation --locked`; confirm RED.
- [x] Implement tagged Serde enums, semantic validation and typed errors; provider data must never panic the core.
- [x] Verify camelCase serialization snapshots and Clippy.
- [x] Commit: `feat(domain): add versioned adaptive translation results`.

### Task 4: Implement local language detection, direction and classification

**Requirements:** `TIT-LANG-01`–`TIT-LANG-07`.

**Files:**
- Create: `src-tauri/src/application/translation/ports.rs`, `language_direction.rs`, `classifier.rs`
- Create: `src-tauri/src/infrastructure/language/mod.rs`, `lingua_detector.rs`
- Create: `src-tauri/tests/fixtures/language_detection.json`
- Test: corresponding modules

- [x] Write failing direction table tests for primary/secondary targets, manual overrides, same-language rejection and uncertainty.
- [x] Write failing 48-language fixture tests with explicit mixed/short uncertain cases and ≥95% expected direction requirement.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml language --locked`; confirm RED.
- [x] Implement `LanguageDetectorPort`, one-time Lingua initialization, high-confidence script rules and confidence/reason output; no network detector exists.
- [x] Implement direction resolver and deterministic intent classifier; explicit overrides always win.
- [x] Run corpus twice to prove determinism and confirm a fake network port receives zero calls.
- [x] Commit: `feat(translation): add local language and intent analysis`.

### Task 5: Implement modes and prompt compilation

**Requirements:** `TIT-MODE-01`–`TIT-MODE-06`.

**Files:**
- Create: `src-tauri/src/domain/translation_mode.rs`, `src-tauri/src/application/translation/prompt_compiler.rs`
- Create: `src-tauri/src/infrastructure/storage/translation_modes.rs`
- Create: `src-tauri/migrations/0001_translation_core.sql` containing the complete immutable core schema for translation modes, provider profiles and the closed `main`/`mini` workspace-role model
- Test: mode/compiler/repository modules

- [x] Write failing seed tests for stable standard/literal/natural/academic/concise IDs, immutable built-ins, custom-copy behavior and version increments.
- [x] Write failing prompt boundary tests proving user instructions cannot replace schema, language or system contract and user正文 is treated as data.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml translation_mode --locked`; confirm RED.
- [x] Add failing schema/state tests proving `window_role` accepts only `main`/`mini` and rejects removed or unknown roles. Seed one `main` workspace profile for this feature: fresh install permits null primary/empty enabled list and returns a configurable unconfigured state; configured state requires a unique valid primary in the ordered enabled list, different primary/secondary languages, valid default mode and optimistic-lock version. The Mini epic later inserts the `mini` profile without changing this role contract.
- [x] Implement the complete `0001` core schema (mode, provider-profile and workspace-profile tables), then mode snapshots, layered `PromptCompiler`, template validation and repository ordering/deletion conflict. Later tasks must not rewrite this applied migration.
- [x] Run temporary-SQLite migration/repository tests and Clippy.
- [x] Commit: `feat(translation): add versioned translation modes`.

### Task 6: Establish SQLite and credential boundaries

**Requirements:** `TIT-AI-03`, storage foundations for `TIT-HIS-*` and `TIT-FAV-*`.

**Files:**
- Create: `src-tauri/src/infrastructure/storage/database.rs`, `migrations.rs`
- Create: `src-tauri/src/infrastructure/credentials/mod.rs`, `system_store.rs`
- Create: `src-tauri/migrations/0002_translation_history.sql`
- Test: storage and credential modules

- [x] Write failing tests for fresh/upgrade migrations, foreign keys, WAL and migration idempotence.
- [x] Write a failing secret-canary test scanning SQLite, DTO JSON, Debug and captured logs.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml storage --locked`; confirm RED.
- [x] Implement database bootstrap before window readiness and `CredentialPort` with redacted secret types; never fall back to plaintext storage.
- [x] Test unavailable credential store, migration failure, secret update/delete and canary absence.
- [x] Commit: `feat(storage): add translation database and credential boundary`.

### Task 7: Implement provider profiles, registry and secure HTTP policy

**Requirements:** `TIT-AI-01`–`TIT-AI-06`, `TIT-AI-09`.

**Files:**
- Create: `src-tauri/src/domain/provider.rs`
- Create: `src-tauri/src/infrastructure/ai/registry.rs`, `http_client.rs`
- Create: `src-tauri/src/infrastructure/storage/provider_profiles.rs`, `workspace_profiles.rs`
- Test: upgrade from the Task 5 `0001` schema without editing its checksum
- Test: corresponding modules

- [x] Write failing profile tests for multiple instances, required primary, disabled profile, URL userinfo/fragment, remote HTTP rejection, loopback HTTP acceptance, timeout and parameters.
- [x] Write failing workspace-profile repository tests for atomic read/update, unique enabled primary, deterministic provider order, target-language pair, default mode, optimistic conflict and invalid referenced/disabled objects.
- [x] Write a failing fresh-install flow: empty database opens, workspace read returns unconfigured state, translation submission returns `primary_missing` without HTTP, and creating the first provider plus selecting it as primary commits atomically.
- [x] Write a failing cross-origin redirect test proving Authorization is never forwarded.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml provider_profile --locked`; confirm RED.
- [x] Implement descriptors, provider/workspace repositories, registry resolution and HTTP policy using the already-created `0001` tables; do not edit `0001`. Return one immutable `WorkspaceProfileSnapshot` for session creation.
- [x] Verify secrets never enter profile rows/DTOs and migrated endpoints are revalidated at request time.
- [x] Commit: `feat(providers): add secure provider profiles and registry`.

### Task 8: Establish the shared AI provider contract

**Requirements:** `TIT-AI-01`, `TIT-AI-04`, `TIT-AI-08`–`TIT-AI-10`, `TIT-RES-01`–`TIT-RES-07`.

**Files:**
- Create: `src-tauri/src/infrastructure/ai/contract_fixtures.rs`, `provider_contract_tests.rs`
- Create: `src-tauri/tests/fixtures/providers/*.json`
- Test: shared adapter contract harness

- [x] Define a failing shared contract for valid adaptive results, auth, rate limit, quota, unavailable model, safety refusal, empty/malformed/schema-invalid/oversized responses, timeout and cancellation.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml provider_contract --locked`; confirm RED because no adapter implements the harness.
- [x] Implement reusable fake request/response builders, error expectations and secret-canary assertions without adding any provider adapter.
- [x] Run the harness unit tests; expect the harness itself GREEN and adapter conformance still pending.
- [x] Commit: `test(providers): define AI provider contract`.

### Task 8.1: Implement OpenAI and OpenAI-compatible adapters

**Files:**
- Create: `src-tauri/src/infrastructure/ai/openai.rs`, `openai_compatible.rs`
- Test: adapters through the Task 8 contract

- [x] Register both unimplemented adapters in the shared contract and run the focused suite; confirm RED for missing request/response/error behavior.
- [x] Implement native OpenAI and the explicitly supported OpenAI-compatible subset, including structure, cancellation and connection capability test.
- [x] Simulate chunked delivery and prove neither adapter returns正文 before full response validation.
- [x] Run both shared contracts, redirect tests and secret-canary scan; expect GREEN.
- [x] Commit: `feat(providers): add OpenAI translation adapters`.

### Task 8.2: Implement the Anthropic native adapter

**Files:**
- Create: `src-tauri/src/infrastructure/ai/anthropic.rs`
- Test: adapter through the Task 8 contract

- [ ] Register the unimplemented Anthropic adapter and run its shared contract; confirm RED.
- [ ] Implement native Messages/structured-output request, complete-response parsing and native error/safety mapping.
- [ ] Run the Anthropic contract, cancellation, malformed response and secret-canary tests; expect GREEN.
- [ ] Commit: `feat(providers): add Anthropic translation adapter`.

### Task 8.3: Implement the Gemini native adapter

**Files:**
- Create: `src-tauri/src/infrastructure/ai/gemini.rs`
- Test: adapter through the Task 8 contract

- [ ] Register the unimplemented Gemini adapter and run its shared contract; confirm RED.
- [ ] Implement native content-generation/response-schema request, complete-response parsing and native error/safety mapping.
- [ ] Run the Gemini contract, cancellation, malformed response and secret-canary tests; expect GREEN.
- [ ] Commit: `feat(providers): add Gemini translation adapter`.

### Task 9: Implement deterministic long-text processing

**Requirements:** `TIT-IN-04`–`TIT-IN-06`, `TIT-RES-05`–`TIT-RES-07`.

**Files:**
- Create: `src-tauri/src/application/translation/chunker.rs`
- Test: same module

- [ ] Write failing boundary tests for paragraphs, oversized sentences, emoji/graphemes, code blocks, lists, index continuity and exact source reconstruction.
- [ ] Write failing orchestration tests proving max concurrency two, source-order output after out-of-order completion, all-or-nothing failure and child cancellation.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml chunk --locked`; confirm RED.
- [ ] Implement deterministic chunk plan, bounded executor and merge validator; only final complete result is persisted/published.
- [ ] Run property tests and Clippy.
- [ ] Commit: `feat(translation): add ordered long-text processing`.

### Task 10: Implement `QuerySupervisor` and `TranslationService`

**Requirements:** `TIT-IN-*`, `TIT-AI-05`–`TIT-AI-10`, `TIT-RETRY-*`, `US-TIT-01`–`US-TIT-08`.

**Files:**
- Create: `src-tauri/src/application/translation/service.rs`, `supervisor.rs`
- Modify: `src-tauri/src/application/translation/mod.rs`
- Test: same modules

- [ ] Write failing request-count tests: one primary + three others must produce exactly one primary call and zero others, including primary failures.
- [ ] Write failing expand/retry tests: first expand creates one attempt; re-expand reuses success; retry creates new ID/version and retains prior success.
- [ ] Add failing retry-snapshot tests: change draft text, languages, mode, service model and parameters before retry; assert the new attempt uses the current explicit configuration and exposes a typed diff against the old attempt.
- [ ] Write failing stale tests: submit A then B and complete A late; A cannot reach B UI/history. Duplicate `submission_id` creates one session/request.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml query_supervisor --locked`; confirm RED.
- [ ] Implement active window slots, persisted workspace snapshot loading, root/child cancellation, session/attempt/generation guards, versioned session workspace snapshots, versioned full attempt request/configuration snapshots, typed diffs, event sequence and post-success persistence hook.
- [ ] Add a restart test: create multiple attempts with changed text/language/mode/model/parameters, reopen SQLite, switch versions and reconstruct the same `ConfigurationDiffDto` without consulting current settings.
- [ ] Test randomized completion order with barriers/notifications, never timing sleeps.
- [ ] Commit: `feat(translation): supervise primary and on-demand AI queries`.

### Task 11: Implement history, retention, privacy and favorites

**Requirements:** `TIT-HIS-01`–`TIT-HIS-07`, `TIT-FAV-01`–`TIT-FAV-05`.

**Files:**
- Create: `src-tauri/migrations/0003_translation_favorites.sql`
- Create: `src-tauri/src/infrastructure/storage/translation_history.rs`, `favorites.rs`
- Test: same modules

- [ ] Write failing transaction tests: success writes session/attempt/result/FTS atomically; failed/cancelled attempts are not success history; write failure leaves no partial row.
- [ ] Write failing policy tests: default saves, history-off does not, privacy never saves/backfills, favorite while history-off writes the minimum stable record.
- [ ] Write failing retention/deletion tests: startup cleanup and policy-update cleanup actually delete expired non-favorites after reopen; periodic cleanup uses one bounded task; all triggers skip favorites; failures emit diagnostic warning; unfavorite preserves history; favorite content deletion requires confirmation.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml history --locked`; confirm RED.
- [ ] Implement parameterized repositories, FTS5 pagination, deterministic sort, transaction rules and a `RetentionService` plus scheduler primitive callable by a future owner; Task 11 must not construct `AppKernel` or start the production periodic task.
- [ ] Run migration/repository tests.
- [ ] Commit: `feat(history): persist translation history and favorites`.

### Task 12: Build typed Tauri IPC and permissions

**Requirements:** All UI-facing requirements and command security.

**Files:**
- Create: `src-tauri/src/application/kernel.rs`, `src-tauri/src/infrastructure/runtime.rs`
- Create: `src-tauri/src/ipc/translation.rs`, `provider_settings.rs`, `translation_modes.rs`, `history.rs`, `favorites.rs`, `clipboard.rs`
- Create/Modify: `src-tauri/src/ipc/dto/*.rs`, `src-tauri/src/ipc/mod.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` for Tauri 2 clipboard-manager plugin
- Generate: `src-tauri/permissions/autogenerated/*.toml`, `src/bridge/generated/*.ts`
- Test: IPC DTO/command tests

- [ ] Write failing tests for discriminated DTO serialization, workspace get/update, mode list/save/delete/reorder, attempt snapshot/configuration diff, event ordering, absence of secret fields, main-window authorization, denial from an unlisted window and constrained clipboard validation. Assert no Floating/side/Settings window label or capability is generated.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml ipc:: --locked`; confirm RED.
- [ ] Implement production `AppKernel` construction in Tauri `setup`: open/migrate SQLite, run startup retention cleanup, construct repositories, credential port, detector, registry, supervisor and translation service, start and retain exactly one 24-hour retention task handle/token, then `.manage(AppKernel)`. Test migration/credential/registry startup failures, retention warning behavior, duplicate setup not creating a second task, and bounded shutdown cancellation.
- [ ] Implement translation/workspace/provider/mode/history/favorite/clipboard commands with `State<AppKernel>` and per-call `Channel`; include detach-window cancellation and typed configuration diff. Do not create speech IPC before Task 17.
- [ ] Initialize Tauri's official clipboard-manager plugin in Rust. Frontend receives no direct plugin permission; `clipboard_write_text` is the only granted write path and is limited to validated plain text.
- [ ] Wire Rust Tauri window events in `lib.rs`/runtime: main hide/destroy and application exit invoke detach/cancel even when WebView cleanup never runs. A Rust integration test closes a window with a blocked fake provider, releases its late response, and proves no Channel success event/history row appears.
- [ ] Register every command in the build manifest, generate allow/deny permissions and grant only explicit allow permissions to `main`.
- [ ] Generate TypeScript bindings twice and run `git diff --exit-code -- src/bridge/generated src-tauri/permissions/autogenerated` after the first generated result is accepted; expect no second-run drift.
- [ ] Run all Rust tests and Clippy.
- [ ] Commit: `feat(ipc): expose typed translation commands`.

### Task 13: Implement frontend bridge and workspace reducer

**Requirements:** Submission, on-demand and stale-session UI foundations.

**Files:**
- Create: `src/bridge/translation.ts`, `providers.ts`, `translationModes.ts`, `workspaceProfile.ts`, `history.ts`, `favorites.ts`, `clipboard.ts`
- Create: `src/features/translation/translationReducer.ts`, `useTranslationSession.ts`
- Test: adjacent `*.test.ts`

- [ ] Write failing reducer sequence tests for prepare/running/success, stale/duplicate events, failure, cancel, retry retaining success, version selection and persistence warning.
- [ ] Write failing bridge request-count tests using mocked invoke/Channel: submit only starts main; expand starts only selected provider; successful re-expand makes no request.
- [ ] Write failing clipboard bridge tests proving only the custom `clipboard_write_text` command is invoked and unsupported content kinds/oversized content are rejected before or by Rust.
- [ ] Write failing workspace/mode bridge tests for typed get/update/list/save/delete/reorder and optimistic conflict normalization.
- [ ] Run `pnpm vitest run src/features/translation src/bridge`; confirm RED.
- [ ] Implement typed bridge and reducer hook using generated DTOs and existing error normalization; forbid `any` and error-string parsing.
- [ ] Run focused tests and `pnpm typecheck`.
- [ ] Commit: `feat(ui): add translation session state`.

### Task 14: Build the complete translation workspace

**Requirements:** `TIT-IN-*`, `TIT-LANG-*`, `TIT-RES-*`, `TIT-RETRY-*`, `US-TIT-01`–`US-TIT-09`.

**Files:**
- Create: `src/features/translation/TranslationWorkspace.tsx`, `translation.css`, `components/*.tsx`, `__tests__/*.test.tsx`
- Modify: `src/app/App.tsx`, `src/i18n/messages.ts`, `src/i18n/catalogs/zh-CN.ts`, `en-US.ts`

- [ ] Write failing IME tests: composition Enter makes zero requests, post-composition Enter makes one, Shift+Enter inserts newline, button is equivalent, empty input validates.
- [ ] Write failing display tests: status/no正文 before success; correct adaptive renderer after success; main expanded, others `NotRequested`; retry preserves old success.
- [ ] Write failing retry-diff tests showing changed text, language, mode, provider or model from the typed attempt diff while the new version generates and after version switching.
- [ ] Write failing clear-action tests: “清空输入” clears only the draft and preserves current result; “清空全部” clears draft and current result but never deletes history.
- [ ] Write failing copy tests for original text, primary translation, one structured block and formatted complete result, preserving Unicode and line breaks through the custom clipboard command.
- [ ] Write failing keyboard, focus, ARIA live, error action and non-color status tests.
- [ ] Write failing lifecycle tests: React cleanup invokes detach/cancel; simulated Tauri window close cancels the active generation; a late response after close cannot render or enter success history.
- [ ] Run `pnpm vitest run src/features/translation`; confirm RED.
- [ ] Implement focused workspace/components with no duplicated business state, retry configuration diff, window detach cleanup, explicit clear/copy formatters and complete typed zh-CN/en-US messages.
- [ ] Run `pnpm check`; capture evidence for word, long text, generating, failure and on-demand states in both locales.
- [ ] Commit: `feat(ui): build adaptive text translation workspace`.

### Task 15: Build Provider and translation-mode settings

**Requirements:** `TIT-AI-01`–`TIT-AI-07`, `TIT-MODE-01`–`TIT-MODE-06`.

**Files:**
- Create: `src/features/provider-settings/*`, `src/features/translation-modes/*`, `src/features/workspace-settings/*`
- Modify: `src/i18n/messages.ts`, `src/i18n/catalogs/*.ts`, `package.json`, `pnpm-lock.yaml`
- Test: feature tests

- [ ] Write failing Provider/workspace tests for multiple profiles, exactly one enabled primary, provider order, primary/secondary languages, default mode, endpoint validation, non-secret save, secret operations, capability test and optimistic conflict.
- [ ] Write failing mode tests for immutable built-ins, custom copy/create/edit/reorder/disable/delete and active-mode replacement.
- [ ] Run focused Vitest suites; confirm RED.
- [ ] Implement descriptor-driven Provider/workspace forms and mode settings through the typed bridges from Task 13; Rust remains authoritative and no stored secret enters DOM.
- [ ] Run `pnpm check` and supply-chain checks.
- [ ] Commit: `feat(settings): manage AI providers and translation modes`.

### Task 16: Build history, favorites and privacy UI

**Requirements:** `TIT-HIS-*`, `TIT-FAV-*`, `US-TIT-10`–`US-TIT-11`.

**Files:**
- Create: `src/features/history/*`, `src/features/favorites/*`
- Modify: `src/features/translation/TranslationWorkspace.tsx`, `src/i18n/messages.ts`, `src/i18n/catalogs/*.ts`
- Test: feature tests

- [ ] Write failing history tests for pagination, filters, detail, single/bulk confirmed delete, retention, history-off and privacy state.
- [ ] Write failing favorite tests for exact attempt version, version switching, unfavorite preserving history, confirmed deletion and retention exclusion.
- [ ] Run focused Vitest suites; confirm RED.
- [ ] Implement paginated workflows. Opening history rehydrates a read-only snapshot without AI calls; rerun remains explicit.
- [ ] Run an integration test proving private正文 never reaches SQLite and favorite-with-history-off stores only the selected stable result.
- [ ] Run `pnpm check`.
- [ ] Commit: `feat(history): add local history privacy and favorites`.

### Task 17: Define the system TTS port, use case and UI contract

**Requirements:** `TIT-RES-10`, `US-TIT-09`.

**Files:**
- Create: `src-tauri/src/application/speech.rs`, `src-tauri/src/infrastructure/speech/mod.rs`
- Create: `src/features/speech/SpeechControls.tsx`, `speechReducer.ts`, contract tests
- Test: Rust port/use-case and injected React bridge tests

- [ ] Write failing port/lifecycle tests for source/result language, play, stop, replacement, unsupported language and window-close policy.
- [ ] Run Rust speech tests and `pnpm vitest run src/features/speech`; confirm RED.
- [ ] Implement stable `SpeechPort`, `SpeechService`, playback state and bridge-injected React controls; do not register production IPC or add `AppKernel` speech state yet.
- [ ] Run shared Rust/React contracts; expect GREEN.
- [ ] Commit: `feat(speech): define system speech contracts`.

### Task 17.1: Implement macOS system TTS

**Files:**
- Create: `src-tauri/src/infrastructure/speech/macos.rs`
- Test: macOS adapter contract and manual fixture

- [ ] Register an unimplemented macOS adapter in the shared contract and confirm RED on macOS.
- [ ] Implement system voice selection, play, stop, replacement and unsupported-language mapping without changing `SpeechPort`.
- [ ] Run macOS adapter tests and manual voice/language/stop matrix; expect GREEN.
- [ ] Commit: `feat(speech): implement macOS system speech`.

### Task 17.2: Implement Windows system TTS

**Files:**
- Create: `src-tauri/src/infrastructure/speech/windows.rs`
- Test: Windows adapter contract and manual fixture

- [ ] Register an unimplemented Windows adapter in the shared contract and confirm RED on Windows.
- [ ] Implement Windows system voice selection, play, stop, replacement and unsupported-language mapping without changing `SpeechPort`.
- [ ] Run Windows adapter tests and manual voice/language/stop matrix; expect GREEN.
- [ ] Commit: `feat(speech): implement Windows system speech`.

### Task 17.3: Implement Linux Speech Dispatcher TTS

**Files:**
- Create: `src-tauri/src/infrastructure/speech/linux.rs`
- Test: Linux adapter contract and X11/Wayland manual fixture

- [ ] Register an unimplemented Linux adapter in the shared contract and confirm RED on Linux.
- [ ] Implement the accepted Rust `tts` adapter backed by Speech Dispatcher, including play, stop, replacement and unsupported-language mapping without changing `SpeechPort`. Declare/document the Speech Dispatcher runtime dependency; when unavailable return the typed actionable error with no command-line or online fallback.
- [ ] Run Linux adapter tests and X11/Wayland voice/language/stop matrix; expect GREEN.
- [ ] Commit: `feat(speech): implement Linux system speech`.

### Task 17.4: Wire TTS into `AppKernel`, typed IPC and workspace

**Files:**
- Create: `src-tauri/src/infrastructure/speech/factory.rs`, `src-tauri/src/ipc/speech.rs`, `src/bridge/speech.ts`
- Modify: `src-tauri/src/application/kernel.rs`, `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`, `src/features/translation/TranslationWorkspace.tsx`, `src/features/speech/*`
- Generate: speech permission and TypeScript DTO files

- [ ] Write failing integration tests proving the platform factory selects exactly one native adapter, only `main` can call play/stop, new playback replaces old, and workspace source/result controls use the correct language.
- [ ] Confirm RED before registration and kernel wiring.
- [ ] Extend production `AppKernel`, register speech commands/permissions, generate DTOs and connect the existing UI contract.
- [ ] Run three-platform builds, shared contracts, generated drift and workspace tests; expect GREEN.
- [ ] Commit: `feat(speech): integrate system translation playback`.

### Task 18: Complete integration, security and release-quality evidence

**Requirements:** Entire PRD Definition of Done.

**Files:**
- Create: `src-tauri/tests/translation_workflow.rs`, `provider_security.rs`, `history_privacy.rs`, `tests/fixtures/translation/*`
- Modify: `.github/workflows/quality.yml`, PRD/baseline traceability documents
- Create: `docs/quality/text-input-translation-evidence.md`

- [ ] Add local-mock end-to-end tests for all result types, primary-only/on-demand, retry/version, cancel/stale, complete-only event, history/favorite/privacy and error categories.
- [ ] Add canary scans across SQLite/log/IPC/diagnostics and prove no traditional translation adapter/config is registered, no unexpanded provider receives traffic, and no removed Floating/side/Settings window role, profile, label or capability exists.
- [ ] Run `pnpm install --frozen-lockfile`, `pnpm check`, Rust fmt/clippy/test, `pnpm tauri build --debug --no-bundle`, `cargo deny`, generated drift and `git diff --check`; every command must exit zero.
- [ ] Execute platform order macOS → Windows → Linux X11/Wayland main-window parity; attach IME, credential, TTS, latency, screenshots and known-limit evidence.
- [ ] Map every `TIT-*`, `EVO-AI-*`, `EVO-HIS-*`, `EVO-FAV-*` to evidence and mark passed only when evidence exists.
- [ ] Commit: `test: verify text input translation workflow`.
- [ ] Use `superpowers:requesting-code-review`, address findings with `superpowers:receiving-code-review`, rerun all verification and publish only through protected PR checks.

## Final acceptance checklist

- [ ] All 18 milestones and their 25 independently committed task units completed in order with focused RED/GREEN evidence.
- [ ] Every PRD requirement has traceability evidence.
- [ ] Primary request count is exactly one and unexpanded other-provider count is zero.
- [ ] Main-provider failure never causes fallback.
- [ ] No正文 appears before a complete validated result.
- [ ] Stale session/attempt results never reach current UI or current success history.
- [ ] History-off, privacy, retention and favorite semantics pass repository and UI tests.
- [ ] No API secret appears in SQLite, generated DTOs, logs or diagnostics.
- [ ] Only `main` and `mini` are accepted as persistent content-window roles; this feature ships only Main UI and leaves no Floating/side/Settings window artifact.
- [ ] macOS, Windows and Linux builds and platform acceptance pass.
- [ ] Feature PR passes all protected-branch checks and documentation matches shipped behavior.

## Requirement traceability map

| Requirement IDs | Primary implementation tasks | Required evidence |
|---|---|---|
| `TIT-IN-01`–`TIT-IN-07` | 9, 10, 13, 14 | normalization/chunk tests, IME/submit/clear UI tests |
| `TIT-LANG-01`–`TIT-LANG-07` | 2, 4, 14 | 48-language corpus, direction tables, override UI |
| `TIT-MODE-01`–`TIT-MODE-06` | 5, 12, 15 | seed/version/compiler/repository/settings tests |
| `TIT-AI-01`–`TIT-AI-10` | 6, 7, 8, 8.1–8.3, 10, 12, 15 | adapter contracts, request counts, secret/permission tests |
| `TIT-RES-01`–`TIT-RES-08` | 3, 8.1–8.3, 9, 10, 12–14 | schema validation, complete-only events, adaptive renderer tests |
| `TIT-RES-09` | 12–14 | constrained clipboard and four copy-format tests |
| `TIT-RES-10` | 17, 17.1–17.4 | shared/platform speech contracts and manual matrix |
| `TIT-RETRY-01`–`TIT-RETRY-05` | 3, 10, 13, 14 | new attempt/version, old-result retention and UI tests |
| `TIT-HIS-01`–`TIT-HIS-07` | 6, 11, 12, 16 | migration/transaction/FTS/policy/privacy tests |
| `TIT-FAV-01`–`TIT-FAV-05` | 6, 11, 12, 16 | stable-reference, delete/retention and UI tests |
| `EVO-AI-01`, `EVO-AI-02` | 5, 8–10, 14, 15 | complete-only and mode evidence |
| `EVO-HIS-01`, `EVO-HIS-02` | 11, 16 | local-history and no-persistence privacy evidence |
| `EVO-FAV-01` | 11, 16 | stable-version favorite evidence |
| All above | 18 | three-platform traceability and release-quality evidence |
