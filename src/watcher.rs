//! File system watcher for Proton Drive Sync

use crate::config::ConfigManager;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::types::{SyncEvent, SyncEventType};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// File watcher
pub struct FileWatcher {
    watcher: Option<notify::RecommendedWatcher>,
    db: Db,
    config: Arc<Mutex<ConfigManager>>,
    running: Arc<Mutex<bool>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(db: Db, config: Arc<Mutex<ConfigManager>>) -> Result<Self> {
        Ok(Self {
            watcher: None,
            db,
            config,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start watching
    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        info!("Starting file watcher");

        let config = self.config.lock().await;
        let sync_dirs = config.get().sync_dirs.clone();
        drop(config);

        // Create watcher
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut watcher =
            notify::recommended_watcher(move |res: std::result::Result<Event, _>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            })?;

        // Watch each sync directory
        for sync_dir in sync_dirs {
            let path = Path::new(&sync_dir.source_path);

            if !path.exists() {
                warn!("Sync directory does not exist: {}", sync_dir.source_path);
                continue;
            }

            watcher.watch(path, RecursiveMode::Recursive)?;
            info!("Watching: {}", sync_dir.source_path);
        }

        self.watcher = Some(watcher);

        // Spawn event handler task
        let db = self.db.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.lock().await {
                match rx.recv().await {
                    Some(event) => {
                        if let Err(e) = Self::handle_event(event, &db, &config).await {
                            error!("Error handling file event: {}", e);
                        }
                    }
                    None => {
                        debug!("Event channel closed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop watching
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping file watcher");
        *self.running.lock().await = false;
        self.watcher = None;
        Ok(())
    }

    /// Handle a file system event
    async fn handle_event(event: Event, db: &Db, config: &Arc<Mutex<ConfigManager>>) -> Result<()> {
        // Skip events with no paths
        if event.paths.is_empty() {
            return Ok(());
        }

        let path = &event.paths[0];

        // Skip temporary files
        if Self::is_temp_file(path) {
            return Ok(());
        }

        // Check if path is in a sync directory
        let cfg = config.lock().await;
        let sync_dir = Self::find_sync_dir(path, cfg.get())?;

        if sync_dir.is_none() {
            return Ok(());
        }

        // Clone the sync dir data so we can drop the lock
        let sync_dir_data = sync_dir.unwrap().clone();
        drop(cfg);

        // Determine event type
        let event_type = match event.kind {
            EventKind::Create(_) => {
                if path.is_dir() {
                    SyncEventType::CreateDir
                } else {
                    SyncEventType::CreateFile
                }
            }
            EventKind::Modify(_) => SyncEventType::Update,
            EventKind::Remove(_) => SyncEventType::Delete,
            _ => {
                debug!("Ignoring event kind: {:?}", event.kind);
                return Ok(());
            }
        };

        // Get relative path
        let base = Path::new(&sync_dir_data.source_path);
        let relative = path
            .strip_prefix(base)
            .map_err(|_| Error::InvalidPath("Path not in sync directory".to_string()))?;

        let local_path = path.to_string_lossy().to_string();
        let remote_path =
            crate::proton::PathUtils::join(&sync_dir_data.remote_root, &relative.to_string_lossy());

        // Check exclusions
        if Self::is_excluded(path, &config.lock().await.get().exclude_patterns) {
            debug!("Path excluded: {}", local_path);
            return Ok(());
        }

        // Generate change token
        let change_token = if event_type != SyncEventType::Delete {
            Self::generate_change_token(path).await?
        } else {
            None
        };

        // Create sync event
        let sync_event = SyncEvent {
            event_type,
            local_path,
            remote_path,
            change_token,
            old_local_path: None,
            old_remote_path: None,
        };

        // Enqueue the job
        db.enqueue_job(&sync_event).await?;

        debug!("Enqueued job: {:?} {:?}", event_type, sync_event.local_path);

        Ok(())
    }

    /// Check if file is temporary
    fn is_temp_file(path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden files (starting with .)
        if file_name.starts_with('.') {
            return true;
        }

        // Skip common temporary patterns
        if file_name.contains('~')
            || file_name.ends_with(".tmp")
            || file_name.ends_with(".swp")
            || file_name.starts_with("._")
        {
            return true;
        }

        false
    }

    /// Find sync directory for a path
    fn find_sync_dir<'a>(
        path: &Path,
        config: &'a crate::types::Config,
    ) -> Result<Option<&'a crate::types::SyncDir>> {
        for sync_dir in &config.sync_dirs {
            let base = Path::new(&sync_dir.source_path);
            if path.starts_with(base) {
                return Ok(Some(sync_dir));
            }
        }
        Ok(None)
    }

    /// Check if path is excluded
    fn is_excluded(path: &Path, patterns: &[crate::types::ExcludePattern]) -> bool {
        let _path_str = path.to_string_lossy();

        for pattern in patterns {
            for glob in &pattern.globs {
                if let Ok(matcher) = glob::Pattern::new(glob) {
                    if matcher.matches_path(path) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Generate change token (mtime:size)
    async fn generate_change_token(path: &Path) -> Result<Option<String>> {
        let metadata = tokio::fs::metadata(path).await?;

        let mtime = metadata
            .modified()
            .map_err(|e| Error::Io(e))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::InvalidPath("Invalid modification time".to_string()))?
            .as_secs();

        let size = metadata.len();

        Ok(Some(format!("{}:{}", mtime, size)))
    }
}

/// File system scanner for reconciliation
pub struct FileScanner;

impl FileScanner {
    /// Scan a directory for changes
    pub async fn scan_directory(
        db: &Db,
        directory: &str,
        remote_root: &str,
        exclusions: &[crate::types::ExcludePattern],
    ) -> Result<usize> {
        info!("Scanning directory: {}", directory);

        let mut count = 0;

        let mut entries = walkdir::WalkDir::new(directory)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !Self::is_excluded(e.path(), exclusions));

        while let Some(Ok(entry)) = entries.next() {
            let path = entry.path();

            // Skip directories themselves (we'll process their contents)
            if path.is_dir() {
                continue;
            }

            let local_path = path.to_string_lossy().to_string();
            let relative = path
                .strip_prefix(directory)
                .map_err(|_| Error::InvalidPath("Path not in base directory".to_string()))?;

            let remote_path =
                crate::proton::PathUtils::join(remote_root, &relative.to_string_lossy());

            // Get current change token
            let change_token = Self::generate_change_token(path).await?;

            // Get stored file state
            let stored_state = db.get_file_state(&local_path).await?;

            // Check if file has changed
            if let Some(stored) = stored_state {
                if stored.change_token == change_token {
                    continue; // No change
                }
            }

            // File is new or changed - enqueue update
            let sync_event = SyncEvent {
                event_type: SyncEventType::Update,
                local_path,
                remote_path,
                change_token: Some(change_token),
                old_local_path: None,
                old_remote_path: None,
            };

            db.enqueue_job(&sync_event).await?;
            count += 1;
        }

        info!("Scan complete: {} changes detected", count);
        Ok(count)
    }

    /// Check if path is excluded
    fn is_excluded(path: &Path, patterns: &[crate::types::ExcludePattern]) -> bool {
        for pattern in patterns {
            for glob in &pattern.globs {
                if let Ok(matcher) = glob::Pattern::new(glob) {
                    if matcher.matches_path(path) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Generate change token
    async fn generate_change_token(path: &Path) -> Result<String> {
        let metadata = tokio::fs::metadata(path).await?;

        let mtime = metadata
            .modified()
            .map_err(|e| Error::Io(e))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::InvalidPath("Invalid modification time".to_string()))?
            .as_secs();

        let size = metadata.len();

        Ok(format!("{}:{}", mtime, size))
    }
}
