use serde_json::{json, Value};
use wbcore::{
    generate_wrapper_stub, BindingTarget, ExecuteRequest, LicenseTier, OwnedToolRuntime,
    OwnedToolRuntimeWithCapabilities, RuntimeOptions,
    ProgressSink, ToolArgs, ToolError, ToolManifest, ToolRuntimeBuilder, ToolRuntimeRegistry,
};
use wblicense_core::{
    verify_signed_entitlement_json, EntitlementCapabilities, LicenseError, VerificationKeyStore,
};
use wbtools_oss::{register_default_tools as register_default_oss_tools, ToolRegistry as OssRegistry};
#[cfg(feature = "pro")]
use wbtools_pro::{register_default_tools as register_default_pro_tools, ToolRegistry as ProRegistry};
#[cfg(feature = "pro")]
use wbtools_pro::licensing::{
    bootstrap_runtime_license_offline, LicensingProviderClient, RuntimeLicenseBootstrapConfig,
    RuntimeLicensePolicy, RuntimeLicenseResolution, ResolvedCapabilities,
};

struct CompositeRegistry {
    oss: OssRegistry,
    #[cfg(feature = "pro")]
    pro: Option<ProRegistry>,
}

impl ToolRuntimeRegistry for CompositeRegistry {
    fn list_tools(&self) -> Vec<wbcore::ToolMetadata> {
        #[cfg(feature = "pro")]
        let mut out = self.oss.list();
        #[cfg(not(feature = "pro"))]
        let out = self.oss.list();
        #[cfg(feature = "pro")]
        if let Some(pro) = &self.pro {
            out.extend(pro.list());
        }
        out
    }

    fn list_manifests(&self) -> Vec<ToolManifest> {
        #[cfg(feature = "pro")]
        let mut out = self.oss.manifests();
        #[cfg(not(feature = "pro"))]
        let out = self.oss.manifests();
        #[cfg(feature = "pro")]
        if let Some(pro) = &self.pro {
            out.extend(pro.manifests());
        }
        out
    }

    fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &wbcore::ToolContext) -> Result<wbcore::ToolRunResult, ToolError> {
        match self.oss.run(id, args, ctx) {
            Ok(v) => Ok(v),
            Err(ToolError::NotFound(_)) => {
                #[cfg(feature = "pro")]
                if let Some(pro) = &self.pro {
                    return pro.run(id, args, ctx);
                }
                Err(ToolError::NotFound(id.to_string()))
            }
            Err(e) => Err(e),
        }
    }
}

fn validate_include_pro(include_pro: bool) -> Result<(), ToolError> {
    #[cfg(feature = "pro")]
    let _ = include_pro;

    #[cfg(not(feature = "pro"))]
    if include_pro {
        return Err(ToolError::InvalidRequest(
            "include_pro=true requested but this build does not include Pro support; rebuild with feature 'pro'".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "pro")]
fn map_provider_error(err: impl std::fmt::Display) -> ToolError {
    ToolError::LicenseDenied(format!("provider bootstrap failed: {err}"))
}

#[cfg(feature = "pro")]
fn parse_policy_from_env() -> RuntimeLicensePolicy {
    match std::env::var("WBW_LICENSE_POLICY") {
        Ok(v) if v.eq_ignore_ascii_case("fail_closed") => RuntimeLicensePolicy::FailClosed,
        _ => RuntimeLicensePolicy::FailOpen,
    }
}

#[cfg(feature = "pro")]
fn parse_lease_seconds_from_env() -> Option<u64> {
    std::env::var("WBW_LICENSE_LEASE_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
}

#[cfg(feature = "pro")]
fn provider_bootstrap_resolution(fallback_tier: LicenseTier) -> Result<Option<RuntimeLicenseResolution>, ToolError> {
    provider_bootstrap_resolution_with(
        fallback_tier,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

#[cfg(feature = "pro")]
fn provider_bootstrap_resolution_with(
    fallback_tier: LicenseTier,
    provider_url_override: Option<&str>,
    floating_license_id_override: Option<&str>,
    machine_id_override: Option<&str>,
    customer_id_override: Option<&str>,
    policy_override: Option<RuntimeLicensePolicy>,
    lease_seconds_override: Option<u64>,
    state_path_override: Option<&str>,
) -> Result<Option<RuntimeLicenseResolution>, ToolError> {
    let provider_url = match provider_url_override {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => match std::env::var("WBW_LICENSE_PROVIDER_URL") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => return Ok(None),
        },
    };

    let mut cfg = RuntimeLicenseBootstrapConfig::default_fail_open("whitebox_next_gen");
    cfg.fallback_tier = fallback_tier;
    cfg.policy = policy_override.unwrap_or_else(parse_policy_from_env);
    cfg.lease_duration_seconds = lease_seconds_override.or_else(parse_lease_seconds_from_env);
    cfg.local_state_path = state_path_override
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var("WBW_LICENSE_STATE_PATH").ok().map(std::path::PathBuf::from));
    cfg.floating_license_id = floating_license_id_override
        .map(ToString::to_string)
        .or_else(|| std::env::var("WBW_FLOATING_LICENSE_ID").ok());
    cfg.floating_machine_id = machine_id_override
        .map(ToString::to_string)
        .or_else(|| std::env::var("WBW_MACHINE_ID").ok())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok());
    cfg.floating_customer_id = customer_id_override
        .map(ToString::to_string)
        .or_else(|| std::env::var("WBW_CUSTOMER_ID").ok());

    let client = LicensingProviderClient::new(provider_url).map_err(map_provider_error)?;
    match client.bootstrap_runtime_license(&cfg, None) {
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            if matches!(cfg.policy, RuntimeLicensePolicy::FailOpen) {
                let offline = bootstrap_runtime_license_offline(&cfg, None, None, current_unix())
                    .map_err(map_provider_error)?;
                Ok(Some(offline))
            } else {
                Err(map_provider_error(e))
            }
        }
    }
}

pub struct RToolRuntime {
    runtime: RuntimeMode,
}

enum RuntimeMode {
    Tier(OwnedToolRuntime<CompositeRegistry>),
    Entitled(OwnedToolRuntimeWithCapabilities<CompositeRegistry, EntitlementCapabilities>),
}

impl Default for RToolRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl RToolRuntime {
    pub fn new() -> Self {
        Self::new_with_options(false, LicenseTier::Open)
            .expect("default runtime construction should not fail")
    }

    #[cfg(feature = "pro")]
    pub fn new_with_options(include_pro: bool, max_tier: LicenseTier) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        if include_pro {
            if let Some(resolution) = provider_bootstrap_resolution(max_tier)? {
                return match resolution.capabilities {
                    ResolvedCapabilities::Entitled(capabilities) => Ok(Self {
                        runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                            CompositeRegistry { oss, pro },
                            RuntimeOptions {
                                max_tier: resolution.effective_tier,
                                expose_locked_tools: false,
                            },
                            capabilities,
                        )),
                    }),
                    ResolvedCapabilities::Fallback(_) => Ok(Self {
                        runtime: RuntimeMode::Tier(
                            ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                                .max_tier(resolution.effective_tier)
                                .build(),
                        ),
                    }),
                };
            }
        }

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                    .max_tier(max_tier)
                    .build(),
            ),
        })
    }

    #[cfg(feature = "pro")]
    pub fn new_with_floating_license_id(
        include_pro: bool,
        fallback_tier: LicenseTier,
        floating_license_id: &str,
        provider_url: Option<&str>,
        machine_id: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        if include_pro {
            if let Some(resolution) = provider_bootstrap_resolution_with(
                fallback_tier,
                provider_url,
                Some(floating_license_id),
                machine_id,
                customer_id,
                None,
                None,
                None,
            )? {
                return match resolution.capabilities {
                    ResolvedCapabilities::Entitled(capabilities) => Ok(Self {
                        runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                            CompositeRegistry { oss, pro },
                            RuntimeOptions {
                                max_tier: resolution.effective_tier,
                                expose_locked_tools: false,
                            },
                            capabilities,
                        )),
                    }),
                    ResolvedCapabilities::Fallback(_) => Ok(Self {
                        runtime: RuntimeMode::Tier(
                            ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                                .max_tier(resolution.effective_tier)
                                .build(),
                        ),
                    }),
                };
            }
        }

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                    .max_tier(fallback_tier)
                    .build(),
            ),
        })
    }

    #[cfg(not(feature = "pro"))]
    pub fn new_with_options(include_pro: bool, max_tier: LicenseTier) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss })
                    .max_tier(max_tier)
                    .build(),
            ),
        })
    }

    #[cfg(feature = "pro")]
    pub fn new_with_entitlement_json(
        include_pro: bool,
        fallback_tier: LicenseTier,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        let capabilities = entitlement_capabilities_from_json(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
        )?;

        Ok(Self {
            runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                CompositeRegistry { oss, pro },
                RuntimeOptions {
                    max_tier: fallback_tier,
                    expose_locked_tools: false,
                },
                capabilities,
            )),
        })
    }

    #[cfg(not(feature = "pro"))]
    pub fn new_with_entitlement_json(
        include_pro: bool,
        fallback_tier: LicenseTier,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let capabilities = entitlement_capabilities_from_json(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
        )?;

        Ok(Self {
            runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                CompositeRegistry { oss },
                RuntimeOptions {
                    max_tier: fallback_tier,
                    expose_locked_tools: false,
                },
                capabilities,
            )),
        })
    }

    pub fn list_tools_json(&self) -> Value {
        let tools: Vec<Value> = self
            .list_visible_manifests()
            .into_iter()
            .map(|m| json!(m))
            .collect();
        Value::Array(tools)
    }

    pub fn run_tool_json(&self, tool_id: &str, args_json: &str) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = self.execute(ExecuteRequest {
            tool_id: tool_id.to_string(),
            args,
        })?;
        Ok(Value::Object(response.outputs.into_iter().collect()))
    }

    pub fn run_tool_json_with_progress(&self, tool_id: &str, args_json: &str) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = self.execute(ExecuteRequest {
            tool_id: tool_id.to_string(),
            args,
        })?;

        Ok(json!({
            "tool_id": response.tool_id,
            "outputs": response.outputs,
            "progress": response.progress,
        }))
    }

    pub fn run_tool_json_with_progress_sink(
        &self,
        tool_id: &str,
        args_json: &str,
        progress: &dyn ProgressSink,
    ) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;
        let response = self.execute_with_progress_sink(
            ExecuteRequest {
                tool_id: tool_id.to_string(),
                args,
            },
            progress,
        )?;

        Ok(json!({
            "tool_id": response.tool_id,
            "outputs": response.outputs,
            "progress": response.progress,
        }))
    }

    fn list_visible_manifests(&self) -> Vec<ToolManifest> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.list_visible_manifests(),
            RuntimeMode::Entitled(runtime) => runtime.list_visible_manifests(),
        }
    }

    fn execute(&self, req: ExecuteRequest) -> Result<wbcore::ExecuteResponse, ToolError> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute(req),
            RuntimeMode::Entitled(runtime) => runtime.execute(req),
        }
    }

    fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<wbcore::ExecuteResponse, ToolError> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute_with_progress_sink(req, progress),
            RuntimeMode::Entitled(runtime) => runtime.execute_with_progress_sink(req, progress),
        }
    }
}

fn entitlement_capabilities_from_json(
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
) -> Result<EntitlementCapabilities, ToolError> {
    let mut key_store = VerificationKeyStore::new();
    key_store
        .insert_base64url_public_key(public_key_kid, public_key_b64url)
        .map_err(map_license_error)?;
    let verified = verify_signed_entitlement_json(signed_entitlement_json, &key_store, current_unix())
        .map_err(map_license_error)?;
    Ok(EntitlementCapabilities::from_verified(&verified, current_unix()))
}

fn current_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn map_license_error(err: LicenseError) -> ToolError {
    ToolError::LicenseDenied(err.to_string())
}

fn read_entitlement_file(path: &str) -> Result<String, ToolError> {
    std::fs::read_to_string(path)
        .map_err(|e| ToolError::InvalidRequest(format!("failed to read entitlement file '{path}': {e}")))
}

fn parse_args_json(args_json: &str) -> Result<ToolArgs, ToolError> {
    let value: Value = serde_json::from_str(args_json)
        .map_err(|e| ToolError::Validation(format!("invalid JSON arguments: {e}")))?;

    let map = value
        .as_object()
        .ok_or_else(|| ToolError::Validation("arguments must be a JSON object".to_string()))?;

    let mut args = ToolArgs::new();
    for (k, v) in map {
        args.insert(k.clone(), v.clone());
    }
    Ok(args)
}

pub fn parse_tier(tier: &str) -> Result<LicenseTier, ToolError> {
    match tier.to_ascii_lowercase().as_str() {
        "open" => Ok(LicenseTier::Open),
        "pro" => Ok(LicenseTier::Pro),
        "enterprise" => Ok(LicenseTier::Enterprise),
        _ => Err(ToolError::InvalidRequest(format!(
            "invalid tier '{tier}', expected open|pro|enterprise"
        ))),
    }
}

pub fn list_tools_json() -> Result<String, ToolError> {
    serde_json::to_string(&RToolRuntime::new().list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn list_tools_json_with_options(include_pro: bool, tier: &str) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let rt = RToolRuntime::new_with_options(include_pro, parsed_tier)?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn list_tools_json_with_entitlement_options(
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let rt = RToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn list_tools_json_with_entitlement_file_options(
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file)?;
    list_tools_json_with_entitlement_options(
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

#[cfg(feature = "pro")]
pub fn list_tools_json_with_floating_license_id_options(
    floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let rt = RToolRuntime::new_with_floating_license_id(
        include_pro,
        parsed_tier,
        floating_license_id,
        provider_url,
        machine_id,
        customer_id,
    )?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

#[cfg(not(feature = "pro"))]
pub fn list_tools_json_with_floating_license_id_options(
    _floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<String, ToolError> {
    list_tools_json_with_options(include_pro, fallback_tier)
}

pub fn run_tool_json(tool_id: &str, args_json: &str) -> Result<String, ToolError> {
    let out = RToolRuntime::new().run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_progress(tool_id: &str, args_json: &str) -> Result<String, ToolError> {
    let out = RToolRuntime::new().run_tool_json_with_progress(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_options(
    tool_id: &str,
    args_json: &str,
    include_pro: bool,
    tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let out = RToolRuntime::new_with_options(include_pro, parsed_tier)?.run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_entitlement_options(
    tool_id: &str,
    args_json: &str,
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let out = RToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )?
    .run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_entitlement_file_options(
    tool_id: &str,
    args_json: &str,
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file)?;
    run_tool_json_with_entitlement_options(
        tool_id,
        args_json,
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

#[cfg(feature = "pro")]
pub fn run_tool_json_with_floating_license_id_options(
    tool_id: &str,
    args_json: &str,
    floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let out = RToolRuntime::new_with_floating_license_id(
        include_pro,
        parsed_tier,
        floating_license_id,
        provider_url,
        machine_id,
        customer_id,
    )?
    .run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

#[cfg(not(feature = "pro"))]
pub fn run_tool_json_with_floating_license_id_options(
    tool_id: &str,
    args_json: &str,
    _floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<String, ToolError> {
    run_tool_json_with_options(tool_id, args_json, include_pro, fallback_tier)
}

pub fn run_tool_json_with_progress_options(
    tool_id: &str,
    args_json: &str,
    include_pro: bool,
    tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let out = RToolRuntime::new_with_options(include_pro, parsed_tier)?
        .run_tool_json_with_progress(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn generate_wrapper_stubs_json_with_options(
    include_pro: bool,
    tier: &str,
    target: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let rt = RToolRuntime::new_with_options(include_pro, parsed_tier)?;
    let target = match target.to_ascii_lowercase().as_str() {
        "python" => BindingTarget::Python,
        "r" => BindingTarget::R,
        _ => {
            return Err(ToolError::InvalidRequest(
                "invalid target, expected 'python' or 'r'".to_string(),
            ))
        }
    };

    let mut stubs = serde_json::Map::new();
    for manifest in rt.list_visible_manifests() {
        stubs.insert(manifest.id.clone(), Value::String(generate_wrapper_stub(&manifest, target)));
    }
    serde_json::to_string(&Value::Object(stubs))
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn generate_r_wrapper_module_with_options(
    include_pro: bool,
    tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let rt = RToolRuntime::new_with_options(include_pro, parsed_tier)?;

    let mut manifests = rt.list_visible_manifests();
    manifests.sort_by(|a, b| a.id.cmp(&b.id));

    let mut out = String::new();
    out.push_str("# Auto-generated wbw_r wrappers\n");
    out.push_str("# Regenerate via generate_r_wrapper_module_with_options(include_pro, tier).\n\n");
    out.push_str("wbw_make_session <- function(floating_license_id = NULL, include_pro = NULL, tier = \"");
    out.push_str(tier);
    out.push_str("\", provider_url = NULL, machine_id = NULL, customer_id = NULL) {\n");
    out.push_str("  resolved_include_pro <- if (is.null(include_pro)) !is.null(floating_license_id) else include_pro\n");
    out.push_str("\n");
    out.push_str("  run_tool <- function(tool_id, args = list()) {\n");
    out.push_str("    args_json <- jsonlite::toJSON(args, auto_unbox = TRUE, null = \"null\")\n");
    out.push_str("    if (!is.null(floating_license_id)) {\n");
    out.push_str("      out_json <- run_tool_json_with_floating_license_id_options(\n");
    out.push_str("        tool_id,\n");
    out.push_str("        args_json,\n");
    out.push_str("        floating_license_id,\n");
    out.push_str("        resolved_include_pro,\n");
    out.push_str("        tier,\n");
    out.push_str("        provider_url,\n");
    out.push_str("        machine_id,\n");
    out.push_str("        customer_id\n");
    out.push_str("      )\n");
    out.push_str("    } else {\n");
    out.push_str("      out_json <- run_tool_json_with_options(tool_id, args_json, resolved_include_pro, tier)\n");
    out.push_str("    }\n");
    out.push_str("    jsonlite::fromJSON(out_json, simplifyVector = FALSE)\n");
    out.push_str("  }\n\n");
    out.push_str("  list_tools <- function() {\n");
    out.push_str("    if (!is.null(floating_license_id)) {\n");
    out.push_str("      out_json <- list_tools_json_with_floating_license_id_options(\n");
    out.push_str("        floating_license_id,\n");
    out.push_str("        resolved_include_pro,\n");
    out.push_str("        tier,\n");
    out.push_str("        provider_url,\n");
    out.push_str("        machine_id,\n");
    out.push_str("        customer_id\n");
    out.push_str("      )\n");
    out.push_str("    } else {\n");
    out.push_str("      out_json <- list_tools_json_with_options(resolved_include_pro, tier)\n");
    out.push_str("    }\n");
    out.push_str("    jsonlite::fromJSON(out_json, simplifyVector = FALSE)\n");
    out.push_str("  }\n\n");
    out.push_str("  session <- new.env(parent = emptyenv())\n");
    out.push_str("  session$run_tool <- run_tool\n");
    out.push_str("  session$list_tools <- list_tools\n");

    for manifest in manifests {
        let fn_name = manifest.id.replace('-', "_");
        out.push_str(&format!(
            "  session${fn_name} <- function(...) {{\n    # {summary}\n    run_tool(\"{tool_id}\", list(...))\n  }}\n",
            fn_name = fn_name,
            summary = manifest.summary.replace('\n', " "),
            tool_id = manifest.id,
        ));
    }

    out.push_str("\n  session\n");
    out.push_str("}\n\n");

    out.push_str(&format!(
        "wbw_run_tool <- function(tool_id, args = list()) {{\n  session <- wbw_make_session(include_pro = {}, tier = \"{}\")\n  session$run_tool(tool_id, args)\n}}\n\n",
        if include_pro { "TRUE" } else { "FALSE" },
        tier,
    ));

    for manifest in rt.list_visible_manifests() {
        let fn_name = manifest.id.replace('-', "_");
        out.push_str(&format!(
            "{fn_name} <- function(...) {{\n  # {summary}\n  session <- wbw_make_session(include_pro = {}, tier = \"{}\")\n  session${fn_name}(...)\n}}\n\n",
            if include_pro { "TRUE" } else { "FALSE" },
            tier,
            fn_name = fn_name,
            summary = manifest.summary.replace('\n', " "),
        ));
    }

    Ok(out)
}

#[cfg(feature = "pro")]
pub fn whitebox_tools(
    floating_license_id: Option<&str>,
    include_pro: Option<bool>,
    tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<RToolRuntime, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let resolved_include_pro = include_pro.unwrap_or(floating_license_id.is_some());

    if let Some(license_id) = floating_license_id {
        RToolRuntime::new_with_floating_license_id(
            resolved_include_pro,
            parsed_tier,
            license_id,
            provider_url,
            machine_id,
            customer_id,
        )
    } else {
        RToolRuntime::new_with_options(resolved_include_pro, parsed_tier)
    }
}

#[cfg(not(feature = "pro"))]
pub fn whitebox_tools(
    _floating_license_id: Option<&str>,
    include_pro: Option<bool>,
    tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<RToolRuntime, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let resolved_include_pro = include_pro.unwrap_or(false);
    RToolRuntime::new_with_options(resolved_include_pro, parsed_tier)
}

mod native_exports {
    use super::*;
    use extendr_api::prelude::{extendr, extendr_module, Nullable};

    fn map_extendr_err(err: ToolError) -> extendr_api::Error {
        extendr_api::Error::Other(err.to_string())
    }

    fn nullable_string_to_option(value: Nullable<String>) -> Option<String> {
        match value {
            Nullable::NotNull(v) => Some(v),
            Nullable::Null => None,
        }
    }

    #[extendr]
    fn list_tools_json() -> extendr_api::Result<String> {
        super::list_tools_json().map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_options(include_pro: bool, tier: &str) -> extendr_api::Result<String> {
        super::list_tools_json_with_options(include_pro, tier).map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_entitlement_options(
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::list_tools_json_with_entitlement_options(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_entitlement_file_options(
        entitlement_file: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::list_tools_json_with_entitlement_file_options(
            entitlement_file,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_floating_license_id_options(
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Nullable<String>,
        machine_id: Nullable<String>,
        customer_id: Nullable<String>,
    ) -> extendr_api::Result<String> {
        let provider_url = nullable_string_to_option(provider_url);
        let machine_id = nullable_string_to_option(machine_id);
        let customer_id = nullable_string_to_option(customer_id);
        super::list_tools_json_with_floating_license_id_options(
            floating_license_id,
            include_pro,
            fallback_tier,
            provider_url.as_deref(),
            machine_id.as_deref(),
            customer_id.as_deref(),
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json(tool_id: &str, args_json: &str) -> extendr_api::Result<String> {
        super::run_tool_json(tool_id, args_json).map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_options(
        tool_id: &str,
        args_json: &str,
        include_pro: bool,
        tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_options(tool_id, args_json, include_pro, tier)
            .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_entitlement_options(
        tool_id: &str,
        args_json: &str,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_entitlement_options(
            tool_id,
            args_json,
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_entitlement_file_options(
        tool_id: &str,
        args_json: &str,
        entitlement_file: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_entitlement_file_options(
            tool_id,
            args_json,
            entitlement_file,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_floating_license_id_options(
        tool_id: &str,
        args_json: &str,
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Nullable<String>,
        machine_id: Nullable<String>,
        customer_id: Nullable<String>,
    ) -> extendr_api::Result<String> {
        let provider_url = nullable_string_to_option(provider_url);
        let machine_id = nullable_string_to_option(machine_id);
        let customer_id = nullable_string_to_option(customer_id);
        super::run_tool_json_with_floating_license_id_options(
            tool_id,
            args_json,
            floating_license_id,
            include_pro,
            fallback_tier,
            provider_url.as_deref(),
            machine_id.as_deref(),
            customer_id.as_deref(),
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn generate_r_wrapper_module_with_options(include_pro: bool, tier: &str) -> extendr_api::Result<String> {
        super::generate_r_wrapper_module_with_options(include_pro, tier).map_err(map_extendr_err)
    }

    extendr_module! {
        mod wbw_r;
        fn list_tools_json;
        fn list_tools_json_with_options;
        fn list_tools_json_with_entitlement_options;
        fn list_tools_json_with_entitlement_file_options;
        fn list_tools_json_with_floating_license_id_options;
        fn run_tool_json;
        fn run_tool_json_with_options;
        fn run_tool_json_with_entitlement_options;
        fn run_tool_json_with_entitlement_file_options;
        fn run_tool_json_with_floating_license_id_options;
        fn generate_r_wrapper_module_with_options;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "pro")]
    use std::sync::OnceLock;
    use std::sync::Mutex;
    use wbcore::ProgressEvent;

    #[derive(Default)]
    struct TestCollectSink {
        events: Mutex<Vec<ProgressEvent>>,
    }

    impl ProgressSink for TestCollectSink {
        fn info(&self, msg: &str) {
            if let Ok(mut events) = self.events.lock() {
                events.push(ProgressEvent::Info(msg.to_string()));
            }
        }

        fn progress(&self, pct: f64) {
            if let Ok(mut events) = self.events.lock() {
                events.push(ProgressEvent::Percent(pct));
            }
        }
    }

    #[cfg(feature = "pro")]
    fn license_env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[cfg(feature = "pro")]
    struct EnvGuard {
        saved: Vec<(String, Option<String>)>,
    }

    #[cfg(feature = "pro")]
    impl EnvGuard {
        fn set(entries: &[(&str, Option<String>)]) -> Self {
            let mut saved = Vec::with_capacity(entries.len());
            for (key, new_val) in entries {
                saved.push(((*key).to_string(), std::env::var(key).ok()));
                match new_val {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
            Self { saved }
        }
    }

    #[cfg(feature = "pro")]
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, old_val) in &self.saved {
                match old_val {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[cfg(feature = "pro")]
    fn unique_missing_state_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "wbw_r_license_state_{}_{}_{}.json",
            tag,
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn list_tools_contains_known_tool() {
        let rt = RToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_add = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("add"));
        assert!(has_add);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn run_tool_json_executes_registry_tool() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let out = rt
            .run_tool_json("raster_power", "{\"input\":[2,3],\"exponent\":2}")
            .expect("tool should run");

        assert_eq!(out.get("result"), Some(&json!([4.0, 9.0])));
    }

    #[test]
    fn pro_tools_hidden_without_pro_options() {
        let rt = RToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(!has_pro);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn run_tool_json_with_progress_returns_progress_events() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let out = rt
            .run_tool_json_with_progress("raster_power", "{\"input\":[2],\"exponent\":2}")
            .expect("tool should run");

        let progress = out
            .get("progress")
            .and_then(Value::as_array)
            .expect("progress should be array");
        assert!(!progress.is_empty());
    }

    #[test]
    #[cfg(feature = "pro")]
    fn run_tool_json_with_progress_sink_emits_live_events() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let sink = TestCollectSink::default();
        let _ = rt
            .run_tool_json_with_progress_sink("raster_power", "{\"input\":[2],\"exponent\":2}", &sink)
            .expect("tool should run");

        let events = sink.events.lock().expect("events lock");
        assert!(!events.is_empty());
    }

    #[test]
    #[cfg(feature = "pro")]
    fn pro_tools_visible_and_runnable_with_pro_options() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(has_pro);

        let out = rt
            .run_tool_json("raster_power", "{\"input\":[2,3],\"exponent\":2}")
            .expect("pro tool should run");
        assert_eq!(out.get("result"), Some(&json!([4.0, 9.0])));
    }

    #[test]
    #[cfg(feature = "pro")]
    fn provider_bootstrap_fail_open_with_missing_state_defaults_to_open() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let state_path = unique_missing_state_path("fail_open");
        let _ = std::fs::remove_file(&state_path);

        let _guard = EnvGuard::set(&[
            ("WBW_LICENSE_PROVIDER_URL", Some("http://127.0.0.1:9".to_string())),
            ("WBW_LICENSE_POLICY", Some("fail_open".to_string())),
            (
                "WBW_LICENSE_STATE_PATH",
                Some(state_path.to_string_lossy().to_string()),
            ),
            ("WBW_LICENSE_LEASE_SECONDS", Some("3600".to_string())),
        ]);

        let rt = RToolRuntime::new_with_options(true, LicenseTier::Open)
            .expect("fail-open bootstrap should not block runtime construction");

        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(!has_pro, "expected OSS/open fallback to hide pro tools");

        let _ = std::fs::remove_file(state_path);
        drop(env_guard);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn provider_bootstrap_fail_closed_with_missing_state_returns_error() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let state_path = unique_missing_state_path("fail_closed");
        let _ = std::fs::remove_file(&state_path);

        let _guard = EnvGuard::set(&[
            ("WBW_LICENSE_PROVIDER_URL", Some("http://127.0.0.1:9".to_string())),
            ("WBW_LICENSE_POLICY", Some("fail_closed".to_string())),
            (
                "WBW_LICENSE_STATE_PATH",
                Some(state_path.to_string_lossy().to_string()),
            ),
            ("WBW_LICENSE_LEASE_SECONDS", Some("3600".to_string())),
        ]);

        match RToolRuntime::new_with_options(true, LicenseTier::Open) {
            Ok(_) => panic!("fail-closed bootstrap should reject runtime construction"),
            Err(err) => assert!(matches!(err, ToolError::LicenseDenied(_))),
        }

        let _ = std::fs::remove_file(state_path);
        drop(env_guard);
    }

    #[test]
    #[cfg(not(feature = "pro"))]
    fn include_pro_rejected_when_pro_feature_disabled() {
        match RToolRuntime::new_with_options(true, LicenseTier::Pro) {
            Ok(_) => panic!("include_pro should be rejected without 'pro' feature"),
            Err(err) => assert!(matches!(err, ToolError::InvalidRequest(_))),
        }
    }

    #[test]
    fn invalid_tier_rejected() {
        let err = parse_tier("gold").expect_err("should reject invalid tier");
        assert!(matches!(err, ToolError::InvalidRequest(_)));
    }

    #[test]
    fn wrapper_stub_generation_returns_known_tool() {
        let txt = generate_wrapper_stubs_json_with_options(false, "open", "r")
            .expect("stub generation should succeed");
        let value: Value = serde_json::from_str(&txt).expect("valid JSON output");
        assert!(value.get("add").is_some());
    }

    #[test]
    fn r_wrapper_module_generation_contains_helper_and_known_tool() {
        let txt = generate_r_wrapper_module_with_options(false, "open")
            .expect("R wrapper module generation should succeed");
        assert!(txt.contains("wbw_make_session <- function"));
        assert!(txt.contains("wbw_run_tool <- function"));
        assert!(txt.contains("run_tool_json_with_options"));
        assert!(txt.contains("list_tools_json_with_options"));
        assert!(txt.contains("session$add <- function"));
        assert!(txt.contains("add <- function"));
    }

    #[test]
    fn r_wrapper_module_generation_matches_manifest_count_and_names() {
        let rt = RToolRuntime::new_with_options(false, LicenseTier::Open)
            .expect("runtime construction should succeed");
        let manifests = rt.list_visible_manifests();

        let txt = generate_r_wrapper_module_with_options(false, "open")
            .expect("R wrapper module generation should succeed");

        let function_def_count = txt.matches(" <- function(").count();
        assert_eq!(
            function_def_count,
            (manifests.len() * 2) + 4,
            "generated module should include session/global wrappers plus helper functions"
        );

        for manifest in manifests {
            let fn_name = manifest.id.replace('-', "_");
            assert!(
                txt.contains(&format!("session${fn_name} <- function(")),
                "missing generated session wrapper for manifest id '{}'",
                manifest.id
            );
            assert!(
                txt.contains(&format!("{fn_name} <- function(")),
                "missing generated global wrapper for manifest id '{}'",
                manifest.id
            );
        }
    }
}
