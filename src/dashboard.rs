//! Web dashboard for Proton Drive Sync

use crate::config::ConfigManager;
use crate::error::Result;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Dashboard state
#[derive(Clone)]
pub struct DashboardState {
    pub config: Arc<Mutex<ConfigManager>>,
}

/// Start the dashboard server
pub async fn start_dashboard(
    config: Arc<Mutex<ConfigManager>>,
    host: String,
    port: u16,
) -> Result<()> {
    let state = DashboardState { config };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/status", get(get_status))
        .route("/api/config", get(get_config))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("Dashboard listening on http://{}:{}", host, port);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Index page handler
async fn index() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// Status API handler
async fn get_status(State(state): State<DashboardState>) -> impl IntoResponse {
    let cfg = state.config.lock().await;
    let config = cfg.get().clone();

    let status = serde_json::json!({
        "sync_dirs": config.sync_dirs.len(),
        "concurrency": config.sync_concurrency,
        "remote_delete_behavior": config.remote_delete_behavior,
    });

    Json(status)
}

/// Config API handler
async fn get_config(State(state): State<DashboardState>) -> impl IntoResponse {
    let cfg = state.config.lock().await;
    let config = cfg.get().clone();
    Json(config)
}

/// Dashboard HTML
pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Proton Drive Sync</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }
        .header {
            background: white;
            padding: 1.5rem 2rem;
            border-radius: 8px;
            margin-bottom: 2rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .header h1 {
            color: #6d4aff;
            font-size: 1.5rem;
        }
        .card {
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            margin-bottom: 1.5rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .card h2 {
            font-size: 1.25rem;
            margin-bottom: 1rem;
            color: #333;
        }
        .stat {
            display: inline-block;
            margin-right: 2rem;
        }
        .stat-value {
            font-size: 2rem;
            font-weight: bold;
            color: #6d4aff;
        }
        .stat-label {
            color: #666;
            font-size: 0.875rem;
        }
        .sync-dir {
            padding: 0.75rem;
            background: #f9f9f9;
            border-radius: 4px;
            margin-bottom: 0.5rem;
        }
        .sync-dir:last-child {
            margin-bottom: 0;
        }
        .sync-dir-path {
            font-family: monospace;
            color: #333;
        }
        .sync-dir-arrow {
            color: #999;
            margin: 0 0.5rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Proton Drive Sync Dashboard</h1>
        </div>

        <div class="card">
            <h2>Status</h2>
            <div class="stat">
                <div class="stat-value" id="sync-dirs-count">-</div>
                <div class="stat-label">Sync Directories</div>
            </div>
            <div class="stat">
                <div class="stat-value" id="concurrency">-</div>
                <div class="stat-label">Concurrency</div>
            </div>
        </div>

        <div class="card">
            <h2>Sync Directories</h2>
            <div id="sync-dirs-list">
                Loading...
            </div>
        </div>
    </div>

    <script>
        async function loadStatus() {
            try {
                const response = await fetch('/api/status');
                const data = await response.json();

                document.getElementById('sync-dirs-count').textContent = data.sync_dirs;
                document.getElementById('concurrency').textContent = data.concurrency;
            } catch (error) {
                console.error('Error loading status:', error);
            }
        }

        async function loadConfig() {
            try {
                const response = await fetch('/api/config');
                const data = await response.json();

                const syncDirsList = document.getElementById('sync-dirs-list');

                if (data.sync_dirs.length === 0) {
                    syncDirsList.innerHTML = '<p style="color: #999;">No sync directories configured</p>';
                    return;
                }

                syncDirsList.innerHTML = data.sync_dirs.map(dir => `
                    <div class="sync-dir">
                        <span class="sync-dir-path">${dir.source_path}</span>
                        <span class="sync-dir-arrow">→</span>
                        <span class="sync-dir-path">${dir.remote_root}</span>
                    </div>
                `).join('');
            } catch (error) {
                console.error('Error loading config:', error);
            }
        }

        loadStatus();
        loadConfig();

        // Refresh every 5 seconds
        setInterval(() => {
            loadStatus();
            loadConfig();
        }, 5000);
    </script>
</body>
</html>
"#;

/// Dashboard index handler with inline HTML
async fn index_html() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}
