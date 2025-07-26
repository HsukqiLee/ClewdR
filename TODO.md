# Architectural Redesign TODO

This document outlines the key areas for improving the architecture of the Clewdr application, ordered by priority.

## 1. Refactor Middleware for Clarity and Safety

- **Goal:** Improve the reliability and maintainability of the middleware layer.
- **Action:**
  - [ ] **Implement a safe context-passing mechanism:**
    - [ ] Create a `ResponseWithContext` wrapper that automatically attaches context data to a `Response` via an extension.
    - [ ] This will eliminate the need for handlers to manually insert context, preventing potential errors.
  - [x] **Unify Claude-related contexts:**
    - [x] Create a `ClaudeContext` enum to represent the different Claude contexts (e.g., `ClaudeWebContext`, `ClaudeCodeContext`).
    - [x] This will simplify the logic in the response middleware, which currently has to check for multiple context types.
  - [ ] **Refactor response middleware:**
    - [ ] Update the response middleware (`to_oai`, `add_usage_info`, `apply_stop_sequences`) to use the new `ResponseWithContext` and `ClaudeContext` types.
    - [ ] This will make the middleware more robust and easier to understand.

## 2. Establish a Comprehensive Testing Strategy

- **Goal:** Create a safety net for refactoring and ensure long-term stability.
- **Action:**
  - [ ] Create a top-level `tests/` directory for integration tests.
  - [ ] Write integration tests that mock the external LLM API endpoints. This allows you to test your application's entire request/response pipeline without making real network calls.
  - [ ] Use tools like `wiremock` or `mockall` to build robust mocks.

## 3. Unify Provider Logic with a Trait-Based System

- **Goal:** Reduce code duplication and make the system more modular and extensible.
- **Action:**
  - [ ] Create a new `src/providers` directory.
  - [ ] Define a generic `LLMProvider` trait in `src/providers/mod.rs` to establish a common interface for all LLM providers.
  - [ ] Refactor existing provider logic (e.g., `claude`, `gemini`) to implement this trait.
  - [ ] Update the router and middleware to use the generic trait, abstracting away provider-specific details.

## 4. Centralize and Simplify Configuration

- **Goal:** Make configuration easier to manage and modify without code changes.
- **Action:**
  - [ ] Consolidate all application settings into a single, strongly-typed `ClewdrConfig` struct.
  - [ ] Use `serde` to load this configuration from a single source (e.g., a `config.toml` file or environment variables).
  - [ ] Include a list or map of provider configurations within the main config struct to easily manage providers.

## 5. Automate Frontend/Backend Type Synchronization

- **Goal:** Eliminate manual synchronization of types between the Rust backend and TypeScript frontend.
- **Action:**
  - [ ] Add the `ts-rs` crate as a build dependency.
  - [ ] Create a `build.rs` script if one doesn't exist.
  - [ ] Annotate your shared Rust API data structures with `#[ts(export)]`.
  - [ ] Configure the build script to automatically generate TypeScript definition files (`.d.ts`) in the `frontend/src/types` directory.

## 6. Refine State Management and Persistence

- **Goal:** Prevent loss of state (API keys, cookies) on application restart.
- **Action:**
  - [ ] Introduce a lightweight persistence layer for state currently managed by in-memory actors.
  - [ ] **Option A (Simple):** On startup, load state from a file (e.g., `state.json`). Use the actor model to manage it in memory for performance, but periodically flush changes back to the file.
  - [ ] **Option B (Robust):** Integrate a simple database like SQLite. Use `sqlx` for asynchronous, type-safe SQL queries. This would provide durable storage for keys, cookies, and user configurations.
