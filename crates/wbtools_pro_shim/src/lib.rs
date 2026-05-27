use std::collections::BTreeMap;
use wbcore::*;

/// Public shim registry matching the private wbtools_pro surface.
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: BTreeMap::new() }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let id = tool.metadata().id.to_string();
        self.tools.insert(id, tool);
    }

    pub fn list(&self) -> Vec<ToolMetadata> {
        self.tools.values().map(|t| t.metadata()).collect()
    }

    pub fn manifests(&self) -> Vec<ToolManifest> {
        self.tools.values().map(|t| t.manifest()).collect()
    }

    pub fn run(&self, id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let tool = self.tools.get(id).ok_or_else(|| ToolError::NotFound(id.to_string()))?;
        let meta = tool.metadata();
        if !ctx.capabilities.has_tool_access(meta.id, meta.license_tier) {
            return Err(ToolError::LicenseDenied(meta.id.to_string()));
        }
        tool.validate(args)?;
        tool.run(args, ctx)
    }
}

impl ToolRuntimeRegistry for ToolRegistry {
    fn list_tools(&self) -> Vec<ToolMetadata> {
        self.list()
    }

    fn list_manifests(&self) -> Vec<ToolManifest> {
        self.manifests()
    }

    fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        self.run(id, args, ctx)
    }
}

/// Shim no-op registration in public OSS builds.
pub fn register_default_tools(_registry: &mut ToolRegistry) {}
