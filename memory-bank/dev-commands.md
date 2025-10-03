# Development Build Commands

This document outlines the commands needed to build and run the project in different environments after the workspace refactoring.

## Development Mode (with Hot-Reload)

For development, you'll need two separate terminal windows.

### Terminal 1: Run the Server

This terminal will run the Axum backend server.

```bash
cargo run -p backend
```

- This command compiles and runs the `backend` crate.
- The server will start on `http://localhost:3000`.
- It will handle API requests but will not serve the client-side application directly in this mode.

### Terminal 2: Run the Client (Trunk)

This terminal will build, watch, and serve the Leptos WASM application. **Run this from the project root.**

```bash
trunk serve
```

- This command now reads the root `Trunk.toml`, which points to the `frontend` crate.
- This command builds the `frontend` crate and starts a development server, usually on `http://localhost:8080`.
- It will automatically rebuild the application whenever you make changes to the `frontend` crate.
- Your browser should be pointed to `http://localhost:8080`. API requests from the client will be sent to the server running on port 3000.

---

## Production Release Build

To create a production-ready build, follow these steps in order.

### 1. Build the Client Application

First, build the optimized WASM and JavaScript assets. **Run this from the project root.**

```bash
trunk build --release
```

- This command compiles the `frontend` crate in release mode, based on the root `Trunk.toml`.
- It places the optimized output files into the root `dist/` directory.

### 2. Build the Server Application

Next, build the optimized server binary.

```bash
cargo build -p backend --release
```

- This command compiles the `backend` crate in release mode.
- The optimized binary will be located at `target/release/backend` (or `backend.exe` on Windows).
