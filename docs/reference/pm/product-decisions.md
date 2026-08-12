# Product Decisions

## 2026-08-12 — Text-input technical design accepted

- **Decision:** The complete text-input translation technical design is accepted and may proceed to sequential implementation.
- **Accepted constraints:** Lingua local detection, direct Rust HTTP adapters, SQLite FTS5, UUID v7 stable identifiers, long-text concurrency of two with all-or-nothing publication, and Linux Speech Dispatcher through the Rust `tts` adapter.
- **Specification:** [`text-input-translation-technical-design.md`](../../architecture/text-input-translation-technical-design.md), Section 20.

## 2026-08-12 — Two-window product model

- **Decision:** Evolish has exactly two persistent content-window roles: `Main` and `Mini`. The former side floating window is a product exclusion.
- **Main window:** Owns manual text input, full results, history, favorites, provider/mode management, settings, and later learning surfaces. Settings are routes/panels inside the main window, not a separate WebView window.
- **Mini window:** Owns automatic selection, shortcut selection, and screenshot-translation results. It appears near the current selection/capture context, stays compact by default, can expose the applicable full result actions, and can hand the current session to the main window.
- **Capture overlay:** A transient, single-purpose capture surface is permitted for region selection but is not a third product/result window and cannot host settings or translation results.
- **Rationale:** One lightweight contextual surface plus one complete workspace removes redundant result-window concepts and gives all capture entry points a predictable destination.

## 2026-08-11 — AI-only translation and Phase 1 favorites

- **Translation boundary:** Translation, definitions, examples, and language analysis use AI providers only. Traditional translation providers such as Apple Translate, DeepL, Google Translate, Youdao Translate, Tencent Translate, Bing Translate, and Baidu Translate are not product dependencies.
- **Provider surface:** Compile-time native adapters cover OpenAI, Anthropic, and Gemini; OpenAI-compatible profiles cover custom endpoints and compatible hosted or local models.
- **Request policy:** A submission calls only the configured primary AI service. Other enabled AI services remain collapsed and make no request until the user explicitly expands them. There is no automatic provider fallback.
- **Presentation policy:** AI response text is shown only after a complete response passes the versioned result-schema validation; streaming body text is not rendered.
- **Local capabilities:** Language detection and text processing may run locally. Text-to-speech uses platform system TTS in the text-input translation workflow.
- **History and favorites:** Successful queries are saved to local history by default and may be favorited in Phase 1. History can be disabled, retention can be configured, and private mode writes no content. Favoriting while history is disabled stores the minimum required result record.
- **Learning boundary:** Favorites reference stable result versions, but knowledge cards, review scheduling, and spaced repetition remain Phase 2 work.
- **Specification:** [`text-input-translation-prd.md`](./text-input-translation-prd.md)

## 2026-08-11 — Primary learning unit

- **Decision:** Any selected content may become a structured knowledge card.
- **Scope:** Cards can represent vocabulary, phrases, sentences, concepts, screenshots, and personal notes. Vocabulary is a card type, not the product boundary.
- **Rationale:** Preserve fast translation and lookup while allowing the product to evolve into a personal AI learning system rather than a vocabulary-only spaced-repetition application.

## 2026-08-11 — Delivery phases

- **Phase 1:** Reproduce the applicable user-facing capability set of Easydict in a cross-platform application, with documented product exclusions for traditional translation providers and documented Evolish extensions for history and favorites.
- **Phase 2:** Develop AI-assisted knowledge cards, review, and the broader learning system.
- **Boundary:** Phase 1 should reserve stable extension points and data boundaries for Phase 2, but must not implement or expose an incomplete review workflow.
- **Rationale:** Establish a mature translation, dictionary, text-selection, OCR, and service-integration foundation before designing the learning loop.

## 2026-08-11 — Phase 1 parity definition

- **Decision:** Easydict parity means matching its user-visible capabilities and outcomes, not copying its pixel-level UI or macOS-specific internal implementation.
- **Completion rule:** Every applicable item in `easydict-feature-baseline.md` must pass platform and behavior acceptance.
- **Platform rule:** Apple Dictionary remains an explicit macOS-only service. Apple Translate is a documented product exclusion under the later AI-only translation decision.
- **Phase 2 boundary:** Phase 1 stores structured query sessions, source context, intent, versioned results, history, and favorites, but exposes no unfinished knowledge-card or review UI.

## 2026-08-11 — Phase 1 platform scope

- **Decision:** Phase 1 targets macOS, Windows, and Linux desktop.
- **Excluded:** iOS and Android product surfaces are not part of Phase 1.
- **Platform-specific behavior:** Apple Dictionary remains macOS-only; Linux Wayland capabilities require an explicit compatibility matrix because the protocol restricts global coordinates and programmatic window positioning. Apple Translate is excluded by the AI-only translation decision.

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
