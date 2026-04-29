# Recipes

Recipes in WbW-QGIS are guided workflow entries that help users launch common multi-tool patterns faster.

A recipe is not a new backend algorithm. It is a curated sequence of existing tools with summary guidance, launch defaults, and tier-aware visibility.

## What Recipes Provide

Recipes provide:

- A short purpose statement.
- A launch tool (the first tool dialog opened when you run the recipe).
- A step list (ordered tool IDs for the workflow).
- Optional input and output hints.
- Tier gating (Open, Pro, or Enterprise).

## Where Recipes Appear in QGIS

Recipes are available in the Whitebox Workflows dock panel under Workflow Recipes.

The panel includes:

- Open Recipe
- Copy Recipe Steps
- Why Is This Locked?
- Open Recipe File
- Reload Recipe File
- Validate Recipe File
- Include locked recipes toggle

## Built-in and User Recipes

WbW-QGIS merges two sources:

1. Built-in recipes shipped with the plugin.
2. User-defined recipes loaded from a local JSON file.

If a user recipe has the same id as a built-in recipe, the user recipe overrides the built-in entry.

## User Recipe File Location

Default file path:

- ~/.whitebox_workflows_qgis/recipes.json

Override path with environment variable:

- WBW_QGIS_USER_RECIPES

When you press Open Recipe File, the plugin creates the file from a template if it does not exist.

## User Recipe File Format

The file may be either:

1. An object with a recipes array.
2. A direct array of recipes.

Each recipe should include:

- id (required)
- tools array (required)

Optional fields:

- title
- summary
- tier (open, pro, enterprise)
- launch_tool
- input_hint
- output_hint

Example:

```json
{
  "recipes": [
    {
      "id": "my_custom_terrain_recipe",
      "title": "My Custom Terrain Recipe",
      "summary": "User-defined recipe example.",
      "tier": "open",
      "launch_tool": "slope",
      "tools": ["slope", "aspect", "hillshade"],
      "input_hint": "Set a DEM raster as the primary input.",
      "output_hint": "Write outputs to a dedicated project output folder."
    }
  ]
}
```

## Validation and Error Reporting

Use Validate Recipe File in the panel to run validation and see a full report.

Validation checks include:

- Entry structure is a JSON object.
- id exists and is non-empty.
- tools exists and is a non-empty array.
- tier value is valid when supplied.

Invalid entries are skipped, while valid entries continue to load.

Warnings include recipe index and, when available, recipe id to speed up fixes.

## Recipe Visibility and Tier Behavior

Recipes are filtered by:

- Runtime tier entitlement.
- Tool availability in the current runtime catalog.
- Include locked recipes panel setting.

When Include locked recipes is enabled, recipes that are not runnable in the current runtime remain visible with lock messaging for discovery.

## Discovery and Sorting

Recipes are shown alphabetically in the panel for easier scanning.

Sorting applies to both built-in and user-defined recipes.

## Recommended Team Practice

For teams, keep a shared recipe JSON under version control and point WBW_QGIS_USER_RECIPES to that path in your local environment setup.

This gives you repeatable, reviewable workflow definitions without modifying plugin source files.
