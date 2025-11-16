// ERP Integration Module
// Exports NetSuite and SAP clients, connection service, sync service, and AI assistant

pub mod netsuite_client;
pub mod sap_client;
pub mod erp_connection_service;
pub mod erp_sync_service;
pub mod erp_ai_assistant_service;

pub use netsuite_client::{NetSuiteClient, NetSuiteConfig, NetSuiteError};
pub use sap_client::{SapClient, SapConfig, SapEnvironment, SapError};
pub use erp_connection_service::{ErpConnectionService, ErpConnection, ErpType, ConnectionStatus, ConflictResolution};
pub use erp_sync_service::{ErpSyncService, SyncResult, SyncDirection};
pub use erp_ai_assistant_service::{
    ErpAiAssistantService,
    MappingSuggestion,
    MappingDiscoveryResponse,
    SyncInsight,
    ConflictResolutionSuggestion,
    ConflictResolutionResponse,
    ConflictData,
};
