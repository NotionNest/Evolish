# Product Decisions

## 2026-08-11 — Primary learning unit

- **Decision:** Any selected content may become a structured knowledge card.
- **Scope:** Cards can represent vocabulary, phrases, sentences, concepts, screenshots, and personal notes. Vocabulary is a card type, not the product boundary.
- **Rationale:** Preserve fast translation and lookup while allowing the product to evolve into a personal AI learning system rather than a vocabulary-only spaced-repetition application.

## 2026-08-11 — Delivery phases

- **Phase 1:** Reproduce the complete user-facing capability set of Easydict in a cross-platform application.
- **Phase 2:** Develop AI-assisted knowledge cards, review, and the broader learning system.
- **Boundary:** Phase 1 should reserve stable extension points and data boundaries for Phase 2, but must not implement or expose an incomplete review workflow.
- **Rationale:** Establish a mature translation, dictionary, text-selection, OCR, and service-integration foundation before designing the learning loop.

## 2026-08-11 — Phase 1 parity definition

- **Decision:** Easydict parity means matching its user-visible capabilities and outcomes, not copying its pixel-level UI or macOS-specific internal implementation.
- **Completion rule:** Every applicable item in `easydict-feature-baseline.md` must pass platform and behavior acceptance.
- **Platform rule:** Apple Dictionary and Apple Translate remain explicit macOS-only services; shared product behavior is implemented through platform adapters.
- **Phase 2 boundary:** Phase 1 stores structured query sessions, source context, intent, and service results, but exposes no unfinished knowledge-card or review UI.

## 2026-08-11 — Phase 1 platform scope

- **Decision:** Phase 1 targets macOS, Windows, and Linux desktop.
- **Excluded:** iOS and Android product surfaces are not part of Phase 1.
- **Platform-specific behavior:** Apple Dictionary and Apple Translate remain macOS-only; Linux Wayland capabilities require an explicit compatibility matrix because the protocol restricts global coordinates and programmatic window positioning.

## 2026-08-11 — Desktop technology stack

- **Decision:** Use Tauri 2 as the desktop shell, Rust as the system/application core, and React + TypeScript + Vite for all desktop UI surfaces.
- **Boundary:** Rust owns business state, platform access, service calls, storage, and secrets. WebViews render UI and communicate through constrained typed IPC.
- **Rationale:** This structure supports deep native integration and low-overhead resident operation while preserving a flexible UI foundation for the later learning system.

## 2026-08-11 — Foundation decisions accepted

- **Storage:** SQLite is the durable store and is accessed exclusively by Rust application/storage code. WebViews never receive direct SQL access.
- **Platform sequence:** Deliver and fully validate macOS first, then Windows, then Linux. X11 targets full Phase 1 parity; Wayland publishes a capability matrix and never claims unsupported positioning or capture behavior.
- **Provider model:** Phase 1 providers are compiled into the application behind capability-specific Rust ports. Dynamic third-party code loading is outside Phase 1.
- **Localization:** The first supported UI locales are `zh-CN` and `en-US`. Message keys are compile-time checked and the locale model carries text direction for later RTL support.
- **Identity:** The application identifier is `io.github.notionnest.evolish`.
- **Distribution:** The repository and future binaries are public. Production distribution requires platform signing, signed updates, database-compatible rollback, SBOM generation, and dependency-license checks.
- **License:** Evolish is licensed under Apache-2.0. Easydict and Pot may inform behavior research, but their GPL implementation code is not copied into Evolish.
- **Feature process:** Before implementing each parity feature, document the observed Pot/Easydict pain point, the Evolish behavior decision, platform scope, and acceptance evidence.
- **Repository governance:** `main` accepts changes through pull requests with required CI, linear history, resolved conversations, and no force push. During the solo-maintainer phase the approval count remains zero.
