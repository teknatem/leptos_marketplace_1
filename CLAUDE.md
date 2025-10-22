# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Leptos marketplace application using a Rust workspace with three crates implementing Domain-Driven Design (DDD) with an aggregate pattern. The project uses:

- **Frontend**: Leptos 0.8 CSR (Client-Side Rendering) with WASM
- **Backend**: Axum REST API with SQLite via Sea-ORM
- **Contracts**: Shared types and domain aggregates

## Architecture

### Workspace Structure

```
crates/
├── frontend/     # Leptos WASM client
├── backend/      # Axum server with REST endpoints
└── contracts/    # Shared domain models and aggregates
```

### Aggregate Pattern

Domain entities use the aggregate pattern with:

- `BaseAggregate<Id>`: Generic base with metadata (created_at, updated_at, is_deleted, version) and event store
- `AggregateRoot` trait: Defines aggregate behavior
- `EntityMetadata`: Tracks entity lifecycle
- Validation via `validate()` method
- `before_write()` hook for pre-persistence logic

Example aggregate: `Connection1CDatabase` in `contracts/src/domain/connection_1c/aggregate.rs`

### Frontend Architecture

Leptos CSR app with global context pattern:

- `AppGlobalContext`: Global reactive state (tabs, sidebar toggles) provided via Leptos context
- Tab-based UI with URL sync (`?active=<tab_key>`)
- Three-panel layout: Left (navbar), Center (tabs), Right (details panel)
- Domain modules in `frontend/src/domain/` contain UI code (list/details views)

### Backend Architecture

- **Service layer** (`domain/*/service.rs`): Business logic, validation, aggregate lifecycle
- **Repository layer** (`domain/*/repository.rs`): Sea-ORM models and database access
- REST endpoints in `main.rs` call service methods
- Database initialized at startup with schema bootstrapping

## Prerequisites

### Frontend Dependencies

Before running the frontend, install the following:

```bash
# Install Trunk (WebAssembly bundler)
cargo install trunk

# Add WebAssembly target
rustup target add wasm32-unknown-unknown
```

## Development Commands

### Build & Run

```bash
# Build all crates
cargo build

# Run backend server (port 3000)
cargo run --bin backend

# Run frontend dev server with Trunk (proxies /api/ to backend)
trunk serve
```

### Frontend Development

Trunk configuration in `Trunk.toml`:
- Target: `crates/frontend/index.html`
- Output: `dist/`
- Proxy: `/api/` → `http://localhost:3000/api/`
- Watch: `./crates/frontend`

### Database

- SQLite database: `target/db/app.db`
- Schema auto-created on backend startup
- Soft-delete pattern (is_deleted flag)

## Domain Examples

Current domain: `connection_1c` (1C database connections)

### Adding a New Domain

1. **Contracts**: Define aggregate in `contracts/src/domain/<domain>/aggregate.rs`
   - Implement `AggregateRoot` trait
   - Add form types and result types for API

2. **Backend**: Create service and repository
   - `backend/src/domain/<domain>/service.rs`: Business logic
   - `backend/src/domain/<domain>/repository.rs`: Sea-ORM model and DB access
   - Add REST endpoints in `backend/src/main.rs`
   - Update schema in `backend/src/shared/data/db.rs`

3. **Frontend**: Add UI module
   - `frontend/src/domain/<domain>/ui/list/`: List view component
   - `frontend/src/domain/<domain>/ui/details/`: Detail/edit view with model and form
   - Register in navbar (`frontend/src/layout/left/navbar.rs`)

## Key Patterns

### Global Context Usage

```rust
let ctx = use_context::<AppGlobalContext>().expect("context not found");
ctx.open_tab("key", "title");
ctx.activate_tab("key");
```

### API Calls (Frontend)

Use `web_sys::window()` with `fetch` or Leptos resources. See `frontend/src/domain/connection_1c/ui/details/model.rs` for examples.

### Service Pattern (Backend)

Service methods:
- Load aggregate from repository
- Apply business logic (e.g., ensure single primary flag)
- Validate via `aggregate.validate()`
- Call `aggregate.before_write()`
- Save via repository

## Testing Endpoints

```bash
# List all connections
curl http://localhost:3000/api/connection_1c

# Get by ID
curl http://localhost:3000/api/connection_1c/1

# Create test data
curl -X POST http://localhost:3000/api/connection_1c/testdata

# Test connection
curl -X POST http://localhost:3000/api/connection_1c/test \
  -H "Content-Type: application/json" \
  -d '{"url":"http://example.com","login":"user","password":"pass"}'
```
