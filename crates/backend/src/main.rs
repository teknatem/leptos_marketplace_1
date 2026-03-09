#![allow(
    clippy::useless_format,
    clippy::unnecessary_map_or,
    clippy::type_complexity,
    clippy::manual_div_ceil,
    clippy::unused_enumerate_index,
    clippy::unnecessary_lazy_evaluations,
    clippy::too_many_arguments,
    clippy::if_same_then_else,
    clippy::unnecessary_cast,
    clippy::redundant_pattern_matching,
    clippy::option_as_ref_deref,
    clippy::derivable_impls
)]

pub mod api;
pub mod dashboards;
pub mod data_schemes;
pub mod data_view;
pub mod domain;
pub mod projections;
pub mod shared;
pub mod system;
pub mod usecases;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::http::{header, Method};
    use axum::middleware;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::services::ServeDir;

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           MARKETPLACE BACKEND STARTING...                ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!("\n");

    // 1. Initialize tracing (системное логирование)
    println!("Step 1: Initializing logging system...");
    match system::tracing::initialize() {
        Ok(_) => println!("✓ Logging system initialized\n"),
        Err(e) => {
            println!("✗ ERROR: Failed to initialize logging: {}\n", e);
            return Err(e);
        }
    }

    // 2. Initialize database (loads config from config.toml)
    println!("Step 2: Initializing database...");
    match shared::data::db::initialize_database().await {
        Ok(_) => println!("✓ Database initialized successfully\n"),
        Err(e) => {
            println!("✗ CRITICAL ERROR: Database initialization failed!");
            println!("   Error: {}\n", e);
            println!("========================================");
            println!("Application cannot start without database.");
            println!("Please check the error messages above.");
            println!("========================================\n");
            return Err(anyhow::anyhow!("db init failed: {e}"));
        }
    }

    // 3. Run database migrations
    println!("Step 3: Running database migrations...");
    match shared::data::migration_runner::run_migrations().await {
        Ok(_) => println!("✓ Database migrations processed\n"),
        Err(e) => {
            println!("✗ ERROR: Database migrations failed: {}\n", e);
            return Err(e);
        }
    }

    // 4. Ensure admin user exists
    println!("Step 4: Checking admin user...");
    match system::initialization::ensure_admin_user_exists().await {
        Ok(_) => println!("✓ Admin user verified\n"),
        Err(e) => {
            println!("✗ ERROR: Admin user check failed: {}\n", e);
            return Err(e);
        }
    }

    // 4.1. Scheduled task worker startup mode
    let scheduled_task_worker_enabled = match shared::config::load_config() {
        Ok(cfg) => cfg.scheduled_tasks.enabled,
        Err(e) => {
            println!(
                "✗ ERROR: Failed to load config for scheduled tasks: {}\n",
                e
            );
            return Err(e);
        }
    };

    println!(
        "Step 5: Scheduled task worker is {} (config.toml -> [scheduled_tasks].enabled)",
        if scheduled_task_worker_enabled {
            "ENABLED"
        } else {
            "DISABLED"
        }
    );

    if scheduled_task_worker_enabled {
        println!("Step 6: Initializing scheduled tasks...");
        let worker = match system::tasks::initialization::initialize_scheduled_tasks().await {
            Ok(w) => {
                println!("✓ Scheduled tasks initialized\n");
                w
            }
            Err(e) => {
                println!("✗ ERROR: Scheduled tasks initialization failed: {}\n", e);
                return Err(e);
            }
        };

        println!("Step 7: Starting background worker...");
        tokio::spawn(async move {
            worker.run_loop().await;
        });
        println!("✓ Background worker started\n");
    } else {
        println!("Step 6: Scheduled task worker disabled by configuration\n");
    }

    // 5. Configure CORS
    println!("Step 8: Configuring CORS...");
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION]);
    println!("✓ CORS configured\n");

    // 6. Build app with routes
    println!("Step 9: Building application routes...");
    let app = axum::Router::new()
        .merge(system::api::configure_system_routes())
        .merge(api::configure_business_routes())
        .fallback_service(ServeDir::new("dist"))
        .layer(middleware::from_fn(
            system::middleware::request_logger::request_logger,
        ))
        .layer(cors);
    println!("✓ Routes configured\n");

    // 7. Start server
    println!("Step 10: Starting HTTP server...");
    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();

    println!("   Attempting to bind to: http://{}", addr);
    tracing::info!("Attempting to bind server to http://{}", addr);

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("✓ Server successfully bound to port 3000\n");
            tracing::info!("Server successfully bound to {}", addr);

            // Вывод информации о доступе к серверу
            println!("========================================");
            println!("  SERVER ACCESS INFORMATION");
            println!("========================================\n");

            println!("✓ Server is accessible at:\n");
            println!("  📍 Local access (on this computer):");
            println!("     http://localhost:3000");
            println!("     http://127.0.0.1:3000\n");

            println!("  📍 Network access (from other computers):");
            println!("     http://<SERVER-IP>:3000");
            println!("     (replace <SERVER-IP> with this computer's IP address)\n");

            println!("  💡 To find this computer's IP address, run:");
            println!("     ipconfig | findstr IPv4\n");

            println!("⚠  TROUBLESHOOTING: If frontend cannot connect:");
            println!("\n  1. Windows Firewall:");
            println!("     Run PowerShell as Administrator:");
            println!("     New-NetFirewallRule -DisplayName \"Backend Port 3000\" `");
            println!("       -Direction Inbound -LocalPort 3000 -Protocol TCP -Action Allow\n");

            println!("  2. Frontend connection:");
            println!("     • Frontend must open backend at http://<SERVER-IP>:3000");
            println!("     • NOT localhost (unless frontend on same server)");
            println!("     • Check browser console for connection errors\n");

            println!("  3. Check if port is accessible:");
            println!("     From another computer, try:");
            println!("     curl http://<SERVER-IP>:3000/api/health");
            println!("     Or open in browser: http://<SERVER-IP>:3000\n");

            println!("========================================\n");

            listener
        }
        Err(e) => {
            println!("✗ CRITICAL ERROR: Cannot bind to port 3000!");
            println!("   Error: {}", e);
            println!("   Error kind: {:?}\n", e.kind());

            if e.kind() == std::io::ErrorKind::AddrInUse {
                println!("========================================");
                println!("Port 3000 is already in use!");
                println!("\nPossible solutions:");
                println!("  1. Stop the other process using port 3000");
                println!("  2. Check Task Manager for other backend.exe");
                println!("  3. Run: netstat -ano | findstr :3000");
                println!("========================================\n");

                tracing::error!(
                    "Error: Port 3000 is already in use. Please ensure no other process is using this port."
                );
            } else {
                println!("========================================");
                println!("Failed to bind to port!");
                println!("\nPossible causes:");
                println!("  - Firewall blocking the port");
                println!("  - Insufficient permissions");
                println!("  - Network configuration issue");
                println!("========================================\n");

                tracing::error!("Failed to bind to port 3000. Error: {}", e);
            }
            return Err(e.into());
        }
    };

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           SERVER STARTED SUCCESSFULLY!                   ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Server listening on: http://{}                ║", addr);
    println!("║  Press Ctrl+C to stop                                    ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!("\n");

    axum::serve(listener, app).await?;

    Ok(())
}
