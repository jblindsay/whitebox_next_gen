# ArcGIS Frontend Strategy (Internal)

Date: 2026-04-16  
Scope: Whitebox Next Gen ArcGIS frontend feasibility, rollout plan, and resourcing model.

Current baseline (confirmed):
1. QGIS plugin is live.
2. Python API is live.
3. R API is live.
4. ArcGIS Pro would be the fourth interface surface.

## Executive Recommendation

Build an ArcGIS Pro frontend, but do it in staged increments and gate each stage by usage and support signals.

Recommended path:
1. Start with an ArcGIS Pro Python toolbox MVP (8-12 weeks).
2. Add a curated, high-value tool subset first (not full parity).
3. Only proceed to a full ArcGIS Pro add-in UX after MVP adoption targets are met.

Rationale:
- Technical feasibility is high for ArcGIS Pro.
- Market upside is meaningful in Esri-heavy enterprise/government segments.
- Engineering/support cost is materially higher than QGIS, so phased de-risking is important.
- Existing QGIS discovery/help/entitlement patterns can be reused, reducing implementation risk versus a greenfield desktop frontend.

## Platform Targeting

Priority order:
1. ArcGIS Pro desktop (primary).
2. ArcGIS Enterprise/Server integration (secondary, after desktop MVP).
3. ArcGIS Online/web app integrations (optional, later-stage UI/distribution channel).
4. ArcMap (do not target).

## Technical Feasibility Summary

What is feasible in ArcGIS Pro:
- Tool discovery and catalog filtering.
- Parameter forms and validation.
- Help integration (local HTML and links).
- Runtime execution with output layer loading.
- Entitlement-aware behavior (open/pro visibility and messaging).

What is harder than QGIS:
- Plugin stack complexity (.NET-first extension model for richer UX).
- Packaging and deployment constraints in managed enterprise environments.
- Broader compatibility testing matrix across ArcGIS Pro versions.

Conclusion: A QGIS-like experience is feasible, but fast parity is not the right first milestone; first prove demand with a focused ArcGIS Pro MVP that reuses the existing three-surface workflow contract standard.

## Architecture Options

### Option A: Python Toolbox First (Recommended)

Description:
- Implement an ArcGIS Pro Python Toolbox (.pyt) that calls whitebox_workflows runtime/session APIs.
- Generate toolbox tools from the same catalog metadata used by Python/R/QGIS frontends.
- Keep UI simple and rely on ArcGIS native geoprocessing panes.

Pros:
- Fastest path to value.
- Lowest initial engineering risk.
- Reuses existing metadata/help infrastructure.

Cons:
- Less differentiated UX than QGIS panel.
- Some advanced panel features (favorites/recents/session banner) likely deferred.

### Option B: .NET Add-in First

Description:
- Build an ArcGIS Pro add-in (C#) with custom dockpane/search/help experience.
- Potentially host Python runtime calls under the hood.

Pros:
- Strongest UX and enterprise polish.
- Better long-term branding and discoverability inside Pro.

Cons:
- Highest upfront complexity.
- Slower time-to-market and larger staffing requirement.

### Option C: Hybrid

Description:
- Deliver Python Toolbox MVP now; begin parallel design of add-in shell.
- Promote high-traffic workflows into add-in surfaces later.

Pros:
- Balanced risk and speed.
- Preserves optionality based on MVP traction.

Cons:
- Requires careful product boundary management to avoid duplicate UX paths.

## 8-12 Week Delivery Plan (Phase 1 MVP)

Target: ArcGIS Pro Python Toolbox integration with curated workflow coverage.

### Weeks 1-2: Discovery and Technical Spike

Deliverables:
- ArcGIS Pro environment baseline (supported versions, Python environment behavior, packaging notes).
- Proof-of-concept call path from ArcGIS Pro tool execution to whitebox runtime session.
- Data I/O compatibility check for key raster/vector/lidar cases.

Exit criteria:
- At least 5 representative tools run end-to-end from Pro.
- No blocking incompatibility in runtime invocation model.

### Weeks 3-4: Catalog Adapter and Tool Generation

Deliverables:
- Adapter from existing tool catalog schema to ArcGIS toolbox parameter schema.
- Auto-generated ArcGIS tool definitions for a curated list (for example 40-80 tools).
- Input/output path handling policy aligned with ArcGIS UX norms.

Exit criteria:
- Generated tools pass basic smoke tests.
- Parameter typing and required/optional behavior validated on a sample set.

### Weeks 5-6: Help, Licensing, and Error UX

Deliverables:
- Help page integration from existing static help assets.
- Open/pro visibility rules mapped to ArcGIS toolbox behavior.
- Clear lock/entitlement messages and downgrade messaging for unavailable pro tools.

Exit criteria:
- Pro-disabled installs show stable, understandable behavior.
- Help access is available for all MVP tools.

### Weeks 7-8: QA, Packaging, and Pilot

Deliverables:
- Regression suite for representative tools and parameter edge cases.
- Installer/distribution guidance for pilot users.
- Pilot readiness package: known limitations, troubleshooting guide, telemetry plan (if used).

Exit criteria:
- Pilot cohort can install and run without developer intervention.
- No critical defects in top workflows.

### Optional Weeks 9-12: Hardening and Expansion

Deliverables:
- Expand tool coverage from curated set toward broader parity.
- Performance tuning for large datasets and long-running tasks.
- Support playbook and triage templates.

Exit criteria:
- Support load stays within planned envelope.
- Clear go/no-go decision for Phase 2 add-in investment.

## Staffing Assumptions

Minimum viable team (Phase 1):
1. 1 geospatial platform engineer (ArcGIS Pro + Python tooling experience).
2. 1 runtime/frontend integration engineer (existing Whitebox catalog/runtime expertise).
3. 0.5 QA engineer (shared) for matrix and regression validation.
4. 0.25 technical writer/devrel support for docs and pilot enablement.

Ideal team (faster execution):
1. 2 engineers full-time (one ArcGIS-focused, one Whitebox integration-focused).
2. 1 QA engineer part-time to full-time during weeks 6-10.
3. 1 product/PM owner part-time for scope control and pilot selection.

## Maintenance Load Estimate

Expected recurring maintenance after MVP:
1. Version compatibility updates for ArcGIS Pro changes.
2. Catalog/schema adapter maintenance as new tools/params are added.
3. Packaging and environment support for enterprise desktops.
4. User support for entitlement and installation issues.

Estimated ongoing load:
- 0.5 to 1.0 engineer equivalent per quarter for stability + compatibility, increasing if full parity is pursued aggressively.

## Marketing and Revenue Hypothesis (Pro Tier)

Potential upside:
1. Access to Esri-centric organizations that are less likely to adopt QGIS-first workflows.
2. Improved enterprise credibility and procurement conversations.
3. Stronger pro-tier upsell opportunities for workflow bundles.

Risks:
1. High support burden if launched with too broad a scope.
2. Slower product velocity if ArcGIS work competes with core runtime priorities.

## Go/No-Go Gates

Proceed from Phase 1 to Phase 2 only if at least two of the following are true:
1. Pilot adoption meets target (define target count and weekly active users before pilot).
2. Support ticket volume remains within agreed threshold.
3. At least one enterprise/pro pipeline cites ArcGIS support as a purchase blocker that is now removed.
4. Engineering estimates for add-in phase fit roadmap without delaying core runtime milestones.

## Suggested Initial Scope (Do First)

Include first:
1. High-value workflows with strong commercial relevance (network analysis, routing, terrain/hydrology bundles, selected remote sensing workflows).
2. Stable tools with clear parameter contracts.
3. Strong docs/help and predictable output behavior.

Defer initially:
1. Full tool parity with QGIS plugin.
2. Advanced custom dockpane UX.
3. Enterprise server publishing automation.

## Decision

Recommended immediate action:
1. Approve a Phase 1 ArcGIS Pro Python Toolbox MVP.
2. Timebox to 8 weeks for first pilot-readiness checkpoint.
3. Evaluate using the gates above before committing to full add-in parity.
