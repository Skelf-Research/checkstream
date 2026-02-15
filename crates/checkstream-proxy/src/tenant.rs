//! Multi-tenant support for CheckStream proxy
//!
//! Provides tenant resolution and per-tenant runtime configuration.

use anyhow::Result;
use axum::http::HeaderMap;
use checkstream_classifiers::ClassifierRegistry;
use checkstream_core::{
    anthropic_adapter, AdapterConfig, ConfigurableAdapter, OpenAiAdapter, StreamAdapter,
};
use checkstream_policy::{ActionExecutor, PolicyEngine};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

use crate::security::{validate_backend_url, UrlValidationConfig};

use crate::config::{MultiTenantConfig, PipelineSettings, ProxyConfig, StreamFormat, TenantConfig};
use crate::proxy::Pipelines;

/// Pre-built runtime state per tenant
///
/// Contains all the pre-initialized components needed to process requests
/// for a specific tenant. This avoids locks in the hot path.
#[derive(Clone)]
pub struct TenantRuntime {
    /// Tenant ID
    pub id: String,

    /// Display name
    pub name: String,

    /// Backend URL for this tenant
    pub backend_url: String,

    /// Pre-built pipelines for the three phases
    pub pipelines: Arc<Pipelines>,

    /// Policy engine for this tenant
    pub policy_engine: Arc<RwLock<PolicyEngine>>,

    /// Action executor
    pub action_executor: Arc<ActionExecutor>,

    /// Stream adapter for parsing backend responses
    pub stream_adapter: Arc<dyn StreamAdapter>,

    /// Token holdback size for streaming
    pub token_holdback: usize,

    /// Maximum buffer capacity
    pub max_buffer_capacity: usize,

    /// Pipeline settings
    pub pipeline_settings: PipelineSettings,
}

impl TenantRuntime {
    /// Create a tenant runtime from tenant configuration
    pub async fn new(
        tenant_config: &TenantConfig,
        default_config: &ProxyConfig,
        shared_registry: Option<Arc<ClassifierRegistry>>,
    ) -> Result<Self> {
        info!("Initializing tenant runtime: {}", tenant_config.id);

        // Validate backend URL to prevent SSRF attacks
        // In production, use strict validation; allow localhost in development via env var
        let url_config = if std::env::var("CHECKSTREAM_DEV_MODE").is_ok() {
            UrlValidationConfig::development()
        } else {
            UrlValidationConfig::default()
        };
        validate_backend_url(&tenant_config.backend_url, &url_config)
            .map_err(|e| anyhow::anyhow!("Invalid backend URL for tenant '{}': {}", tenant_config.id, e))?;

        // Use shared registry or load tenant-specific classifiers
        let registry = if let Some(shared) = shared_registry {
            shared
        } else {
            let classifiers_path = tenant_config
                .classifiers_config
                .as_ref()
                .unwrap_or(&default_config.classifiers_config);
            Arc::new(ClassifierRegistry::from_file(classifiers_path).await?)
        };

        // Determine pipeline settings (tenant override or default)
        let pipeline_settings = tenant_config
            .pipelines
            .clone()
            .unwrap_or_else(|| default_config.pipelines.clone());

        // Build pipelines
        let pipelines = Self::build_pipelines(&pipeline_settings, registry.as_ref())?;

        // Load policy engine
        let policy_engine = Self::load_policy_engine(&tenant_config.policy_path)?;
        info!(
            "Tenant {} loaded {} policies",
            tenant_config.id,
            policy_engine.policies().len()
        );

        // Create stream adapter based on format
        let stream_adapter = create_stream_adapter(&tenant_config.stream_format);

        Ok(Self {
            id: tenant_config.id.clone(),
            name: tenant_config.name.clone(),
            backend_url: tenant_config.backend_url.clone(),
            pipelines: Arc::new(pipelines),
            policy_engine: Arc::new(RwLock::new(policy_engine)),
            action_executor: Arc::new(ActionExecutor::new()),
            stream_adapter,
            token_holdback: tenant_config
                .token_holdback
                .unwrap_or(default_config.token_holdback),
            max_buffer_capacity: tenant_config
                .max_buffer_capacity
                .unwrap_or(default_config.max_buffer_capacity),
            pipeline_settings,
        })
    }

    /// Create a tenant runtime from the default proxy configuration (backward compat)
    pub async fn from_proxy_config(config: &ProxyConfig) -> Result<Self> {
        info!("Initializing default tenant runtime");

        // Validate backend URL to prevent SSRF attacks
        let url_config = if std::env::var("CHECKSTREAM_DEV_MODE").is_ok() {
            UrlValidationConfig::development()
        } else {
            UrlValidationConfig::default()
        };
        validate_backend_url(&config.backend_url, &url_config)
            .map_err(|e| anyhow::anyhow!("Invalid backend URL: {}", e))?;

        // Load classifier registry
        let registry = ClassifierRegistry::from_file(&config.classifiers_config).await?;

        // Build pipelines
        let pipelines = Self::build_pipelines(&config.pipelines, &registry)?;

        // Load policy engine
        let policy_engine = Self::load_policy_engine(&config.policy_path)?;

        Ok(Self {
            id: "_default".to_string(),
            name: "Default Tenant".to_string(),
            backend_url: config.backend_url.clone(),
            pipelines: Arc::new(pipelines),
            policy_engine: Arc::new(RwLock::new(policy_engine)),
            action_executor: Arc::new(ActionExecutor::new()),
            stream_adapter: Arc::new(OpenAiAdapter::new()),
            token_holdback: config.token_holdback,
            max_buffer_capacity: config.max_buffer_capacity,
            pipeline_settings: config.pipelines.clone(),
        })
    }

    /// Build pipelines from settings
    fn build_pipelines(
        settings: &PipelineSettings,
        registry: &ClassifierRegistry,
    ) -> Result<Pipelines> {
        let ingress = registry.build_pipeline(&settings.ingress_pipeline)?;
        let midstream = registry.build_pipeline(&settings.midstream_pipeline)?;
        let egress = registry.build_pipeline(&settings.egress_pipeline)?;

        Ok(Pipelines {
            ingress,
            midstream,
            egress,
        })
    }

    /// Load policy engine from path
    fn load_policy_engine(policy_path: &str) -> Result<PolicyEngine> {
        let mut engine = PolicyEngine::new();

        let path = std::path::Path::new(policy_path);
        if path.exists() {
            if path.is_file() {
                engine.load_policy(policy_path)?;
            } else if path.is_dir() {
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path
                        .extension()
                        .is_some_and(|ext| ext == "yaml" || ext == "yml")
                    {
                        if let Err(e) = engine.load_policy(&path) {
                            warn!("Failed to load policy {:?}: {}", path, e);
                        }
                    }
                }
            }
        } else {
            info!(
                "Policy path does not exist, using empty policy engine: {}",
                policy_path
            );
        }

        Ok(engine)
    }
}

/// Create a stream adapter based on the stream format configuration
fn create_stream_adapter(format: &StreamFormat) -> Arc<dyn StreamAdapter> {
    match format {
        StreamFormat::OpenAi => Arc::new(OpenAiAdapter::new()),
        StreamFormat::Anthropic => Arc::new(anthropic_adapter()),
        StreamFormat::Custom(config) => Arc::new(ConfigurableAdapter::new(AdapterConfig {
            name: "custom".to_string(),
            format: config.format.clone(),
            content_path: config.content_path.clone(),
            done_marker: config.done_marker.clone(),
            content_events: config.content_events.clone(),
            finish_reason_path: None,
        })),
    }
}

/// Resolves tenant from incoming requests
///
/// Resolution order:
/// 1. X-Tenant-ID header
/// 2. Path prefix (e.g., /tenant-id/v1/chat/completions)
/// 3. API key mapping
/// 4. Default tenant
pub struct TenantResolver {
    /// Named tenants
    tenants: HashMap<String, Arc<TenantRuntime>>,

    /// API key to tenant ID mapping
    api_key_index: HashMap<String, String>,

    /// Default tenant (used when no tenant is resolved)
    default_tenant: Arc<TenantRuntime>,
}

impl TenantResolver {
    /// Create a new tenant resolver
    pub fn new(
        tenants: HashMap<String, Arc<TenantRuntime>>,
        api_key_index: HashMap<String, String>,
        default_tenant: Arc<TenantRuntime>,
    ) -> Self {
        Self {
            tenants,
            api_key_index,
            default_tenant,
        }
    }

    /// Build from multi-tenant configuration
    pub async fn from_config(config: &MultiTenantConfig) -> Result<Self> {
        // Build default tenant from existing config
        let default_tenant = Arc::new(TenantRuntime::from_proxy_config(&config.default).await?);

        // Build named tenants
        let mut tenants = HashMap::new();
        let mut api_key_index = HashMap::new();

        for (id, tenant_config) in &config.tenants {
            let runtime = TenantRuntime::new(tenant_config, &config.default, None).await?;

            // Build API key index
            for key in &tenant_config.api_keys {
                api_key_index.insert(key.clone(), id.clone());
            }

            tenants.insert(id.clone(), Arc::new(runtime));
        }

        info!("TenantResolver initialized with {} tenants", tenants.len());

        Ok(Self {
            tenants,
            api_key_index,
            default_tenant,
        })
    }

    /// Resolve tenant from request
    ///
    /// Priority:
    /// 1. X-Tenant-ID header
    /// 2. Path prefix
    /// 3. API key mapping
    /// 4. Default tenant
    pub fn resolve(&self, headers: &HeaderMap, path: &str) -> Arc<TenantRuntime> {
        // 1. Check X-Tenant-ID header
        if let Some(tenant_id) = headers.get("x-tenant-id") {
            if let Ok(id) = tenant_id.to_str() {
                if let Some(tenant) = self.tenants.get(id) {
                    debug!("Resolved tenant from header");
                    return tenant.clone();
                }
                // Don't log the tenant ID to prevent enumeration attacks
                debug!("Tenant resolution failed, using default");
            }
        }

        // 2. Check path prefix: /tenant-id/v1/...
        if let Some(tenant_id) = extract_path_tenant(path) {
            if let Some(tenant) = self.tenants.get(&tenant_id) {
                debug!("Resolved tenant from path: {}", tenant_id);
                return tenant.clone();
            }
            // Don't warn here - might just be a normal path
        }

        // 3. Check API key mapping
        if let Some(auth) = headers.get("authorization") {
            if let Ok(auth_str) = auth.to_str() {
                if let Some(api_key) = extract_api_key(auth_str) {
                    if let Some(tenant_id) = self.api_key_index.get(api_key) {
                        if let Some(tenant) = self.tenants.get(tenant_id) {
                            debug!("Resolved tenant from API key");
                            return tenant.clone();
                        }
                    }
                }
            }
        }

        // 4. Return default tenant
        debug!("Using default tenant");
        self.default_tenant.clone()
    }

    /// Get tenant by ID directly
    pub fn get(&self, tenant_id: &str) -> Option<Arc<TenantRuntime>> {
        self.tenants.get(tenant_id).cloned()
    }

    /// Get the default tenant
    pub fn default_tenant(&self) -> &Arc<TenantRuntime> {
        &self.default_tenant
    }

    /// List all tenant IDs
    pub fn list_tenants(&self) -> Vec<&str> {
        self.tenants.keys().map(|s| s.as_str()).collect()
    }

    /// Check if multi-tenant mode is enabled
    pub fn is_multi_tenant(&self) -> bool {
        !self.tenants.is_empty()
    }
}

/// Extract tenant ID from path prefix
///
/// Expects paths like: /tenant-id/v1/chat/completions
/// Returns the tenant-id portion if present and not "v1"
fn extract_path_tenant(path: &str) -> Option<String> {
    let path = path.trim_start_matches('/');
    let parts: Vec<&str> = path.split('/').collect();

    // Need at least 2 parts: tenant-id and v1
    if parts.len() >= 2 && parts[1] == "v1" && parts[0] != "v1" {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Extract API key from Authorization header
///
/// Supports:
/// - Bearer token: "Bearer sk-..."
/// - Plain API key: "sk-..."
fn extract_api_key(auth: &str) -> Option<&str> {
    let auth = auth.trim();
    if let Some(key) = auth.strip_prefix("Bearer ") {
        Some(key.trim())
    } else if auth.starts_with("sk-") || auth.starts_with("key-") {
        Some(auth)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_tenant() {
        assert_eq!(
            extract_path_tenant("/my-tenant/v1/chat/completions"),
            Some("my-tenant".to_string())
        );
        assert_eq!(extract_path_tenant("/v1/chat/completions"), None);
        assert_eq!(
            extract_path_tenant("my-tenant/v1/chat"),
            Some("my-tenant".to_string())
        );
        assert_eq!(extract_path_tenant("/single"), None);
    }

    #[test]
    fn test_extract_api_key() {
        assert_eq!(extract_api_key("Bearer sk-test123"), Some("sk-test123"));
        assert_eq!(extract_api_key("sk-test123"), Some("sk-test123"));
        assert_eq!(extract_api_key("key-test123"), Some("key-test123"));
        assert_eq!(extract_api_key("invalid"), None);
    }
}
