pub mod models;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

use crate::error::AppError;
use models::Workbook;

// TODO: Import from ouroboros-sheet-db once implementation is complete
// use ouroboros_sheet_db::CellStore;
// use ouroboros_kv::KvEngine;

/// Database connection wrapper
///
/// Currently uses in-memory storage as a stub implementation.
/// TODO: Replace with actual ouroboros-sheet-db implementation
#[derive(Clone)]
pub struct Database {
    // Temporary in-memory storage for workbooks
    workbooks: Arc<RwLock<HashMap<Uuid, Workbook>>>,
    // Temporary in-memory storage for workbook content
    contents: Arc<RwLock<HashMap<Uuid, serde_json::Value>>>,
    // Temporary in-memory storage for CRDT updates
    yrs_updates: Arc<RwLock<HashMap<Uuid, Vec<Vec<u8>>>>>,

    // TODO: Add CellStore once implementation is complete
    // cell_store: Arc<CellStore>,
}

impl Database {
    /// Connect to the database
    ///
    /// TODO: Replace with actual ouroboros-sheet-db initialization
    /// Currently creates in-memory storage as stub
    pub async fn connect(_database_path: &str) -> anyhow::Result<Self> {
        tracing::warn!("Using in-memory stub storage - data will not persist!");

        // TODO: Initialize actual storage
        // let kv_engine = KvEngine::new(database_path).await?;
        // let cell_store = CellStore::new(database_path, "default").await?;

        Ok(Self {
            workbooks: Arc::new(RwLock::new(HashMap::new())),
            contents: Arc::new(RwLock::new(HashMap::new())),
            yrs_updates: Arc::new(RwLock::new(HashMap::new())),
            // cell_store: Arc::new(cell_store),
        })
    }

    /// Initialize database (replaces migrate)
    ///
    /// TODO: Implement proper initialization with ouroboros-sheet-db
    pub async fn migrate(&self) -> anyhow::Result<()> {
        tracing::info!("Database initialized (in-memory stub)");
        // TODO: Initialize schema, recover from WAL, etc.
        Ok(())
    }

    /// List all workbooks
    ///
    /// TODO: Store workbook metadata in ouroboros-kv
    pub async fn list_workbooks(&self) -> Result<Vec<Workbook>, AppError> {
        let workbooks = self.workbooks.read().await;
        let mut result: Vec<_> = workbooks.values().cloned().collect();
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(result)
    }

    /// Create a new workbook
    ///
    /// TODO: Store in ouroboros-kv with proper serialization
    pub async fn create_workbook(&self, name: &str) -> Result<Workbook, AppError> {
        let workbook = Workbook {
            id: Uuid::new_v4(),
            name: name.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.workbooks.write().await.insert(workbook.id, workbook.clone());
        Ok(workbook)
    }

    /// Get a workbook by ID
    ///
    /// TODO: Query from ouroboros-kv
    pub async fn get_workbook(&self, id: Uuid) -> Result<Option<Workbook>, AppError> {
        Ok(self.workbooks.read().await.get(&id).cloned())
    }

    /// Update a workbook
    ///
    /// TODO: Update in ouroboros-kv with atomic CAS operation
    pub async fn update_workbook(&self, id: Uuid, name: &str) -> Result<Workbook, AppError> {
        let mut workbooks = self.workbooks.write().await;

        if let Some(workbook) = workbooks.get_mut(&id) {
            workbook.name = name.to_string();
            workbook.updated_at = Utc::now();
            Ok(workbook.clone())
        } else {
            Err(AppError::NotFound(format!("Workbook {} not found", id)))
        }
    }

    /// Delete a workbook
    ///
    /// TODO: Delete from ouroboros-kv and cleanup associated cell data
    pub async fn delete_workbook(&self, id: Uuid) -> Result<(), AppError> {
        self.workbooks.write().await.remove(&id);
        self.contents.write().await.remove(&id);
        self.yrs_updates.write().await.remove(&id);
        Ok(())
    }

    /// Get workbook content as JSON
    ///
    /// TODO: Query cells from CellStore and serialize to JSON
    pub async fn get_workbook_content(
        &self,
        id: Uuid,
    ) -> Result<Option<serde_json::Value>, AppError> {
        Ok(self.contents.read().await.get(&id).cloned())
    }

    /// Save workbook content as JSON
    ///
    /// TODO: Parse JSON and store cells in CellStore
    pub async fn save_workbook_content(
        &self,
        id: Uuid,
        content: &serde_json::Value,
    ) -> Result<(), AppError> {
        self.contents.write().await.insert(id, content.clone());

        // Update workbook timestamp
        if let Some(workbook) = self.workbooks.write().await.get_mut(&id) {
            workbook.updated_at = Utc::now();
        }

        Ok(())
    }

    /// Store a CRDT update
    ///
    /// TODO: Store in ouroboros-kv with versioning
    pub async fn store_yrs_update(&self, workbook_id: Uuid, update: &[u8]) -> Result<(), AppError> {
        let mut updates = self.yrs_updates.write().await;
        updates.entry(workbook_id)
            .or_insert_with(Vec::new)
            .push(update.to_vec());
        Ok(())
    }

    /// Get all CRDT updates for a workbook
    ///
    /// TODO: Query from ouroboros-kv in chronological order
    pub async fn get_yrs_updates(&self, workbook_id: Uuid) -> Result<Vec<Vec<u8>>, AppError> {
        Ok(self.yrs_updates.read().await
            .get(&workbook_id)
            .cloned()
            .unwrap_or_default())
    }
}
