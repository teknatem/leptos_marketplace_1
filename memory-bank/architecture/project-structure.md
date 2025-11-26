# Project Structure Overview

This document outlines the workspace-based structure of the project, designed for clear separation of concerns, scalability, and adherence to DDD and Vertical Slice Architecture (VSA) principles.

## Cargo Workspace

The project is organized as a Cargo workspace with three primary crates:

- **`/app`**: The client-side application built with Leptos. This crate compiles to WebAssembly (WASM) and contains all UI components, client-side logic, and views.
- **`/server`**: The backend server application built with Axum. This crate compiles to a native binary and is responsible for handling API requests, interacting with the database, and serving the compiled client application in production.
- **`/api`**: A shared library crate that defines the data contract between the client and the server. It contains all Data Transfer Objects (DTOs), shared structs, and enums. This crate is a dependency for both `app` and `server`, ensuring type safety across the entire stack.

## Directory Structure & VSA

The structure is designed to group code by feature (Vertical Slice) rather than by technical layer (e.g., controllers, services).

```
leptos_marketplace_1/
├── 📂 app/                # Client (Leptos WASM)
│   ├── 📄 Cargo.toml
│   ├── 📄 Trunk.toml        # Trunk configuration file
│   └── 📂 src/
│       └── 📂 domain/
│           └── 📂 [feature_name]/
│               ├── 📄 view.rs
│               └── 📄 model.rs
│
├── 📂 server/             # Backend (Axum)
│   ├── 📄 Cargo.toml
│   └── 📂 src/
│       └── 📂 domain/
│           └── 📂 [feature_name]/
│               ├── 📄 endpoint.rs
│               └── 📄 repository.rs
│
├── 📂 api/                # Shared DTOs and types
│   ├── 📄 Cargo.toml
│   └── 📂 src/
│       └── 📂 domain/
│           └── 📂 [feature_name]/
│               └── 📄 aggregate.rs
│
├── 📄 Cargo.toml          # Root workspace configuration
└── 📂 dist/                # Output directory for the compiled client app
```

### How It Works:

1.  **Shared Contract (`/api`)**: When you need a new data type, like `ProductDto`, you define it in `/api/src/domain/products/aggregate.rs`. You add `#[derive(Serialize, Deserialize, Clone)]` to it.
2.  **Backend Implementation (`/server`)**: The server's API endpoint in `/server/src/domain/products/endpoint.rs` will use `ProductDto` from the `api` crate for its request/response bodies. The business logic and database interaction are handled in `repository.rs`.
3.  **Frontend Implementation (`/app`)**: The client-side components in `/app/src/domain/products/view.rs` will also import `ProductDto` from the `api` crate to make typed requests to the server and manage state.

This approach ensures that if you change a shared data structure, the Rust compiler will require you to update both the client and the server, preventing entire classes of bugs.
