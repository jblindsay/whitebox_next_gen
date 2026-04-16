"""
Legacy help file integration for Whitebox Workflows QGIS plugin.

Parses HTML help files from the legacy plugin to extract tool descriptions
and parameter documentation for enriching the QGIS parameter dialogs.
"""

import os
import re
import html
from pathlib import Path
from typing import Optional
from html.parser import HTMLParser


class _HelpHTMLParser(HTMLParser):
    """Parse legacy help HTML files to extract sections."""

    def __init__(self):
        super().__init__()
        self.in_section = None
        self.current_content = []
        self.sections = {}
        self.in_tag = False
        self.tag_stack = []

    def handle_starttag(self, tag: str, attrs):
        if tag in ("h2", "h3"):
            # End previous section if any
            if self.in_section:
                text = "".join(self.current_content).strip()
                self.sections[self.in_section] = text
                self.current_content = []
            self.in_section = None
            self.tag_stack.append(tag)
        elif tag in ("p", "code", "strong", "em", "a", "br", "img", "li", "ul"):
            # Track which tags are open for context
            self.tag_stack.append(tag)
        elif tag == "img":
            # Skip images
            pass

    def handle_endtag(self, tag: str):
        if tag in ("h2", "h3", "p", "code", "strong", "em", "a", "li", "ul"):
            if self.tag_stack and self.tag_stack[-1] == tag:
                self.tag_stack.pop()
        if tag == "p":
            self.current_content.append("\n")

    def handle_data(self, data: str):
        if data.strip():
            # Only add data after we've seen a heading
            if self.in_section is not None:
                self.current_content.append(data)
            elif self.tag_stack and self.tag_stack[0] in ("h2", "h3"):
                # This is the heading text
                heading = data.strip()
                if self.in_section is None and heading and heading[0].isupper():
                    self.in_section = heading


class ToolHelpProvider:
    """Provides access to legacy help files for tools."""

    def __init__(self, help_dir: str = None):
        """Initialize with path to legacy help directory.

        Args:
            help_dir: Path to directory containing HTML help files.
                     If None, looks in standard locations.
        """
        if help_dir is None:
            # Try standard locations
            candidates = [
                Path.home()
                / "Documents/programming/Rust/whitebox_workflows/wbw_qgis/help",
                Path(__file__).parent / "help",
            ]
            for cand in candidates:
                if cand.exists():
                    help_dir = str(cand)
                    break

        self.help_dir = help_dir
        self._cache = {}

    def has_help(self, tool_id: str) -> bool:
        """Check if help file exists for tool."""
        if not self.help_dir:
            return False
        help_file = Path(self.help_dir) / f"{tool_id}.html"
        return help_file.exists()

    def get_tool_description(self, tool_id: str) -> Optional[str]:
        """Extract main tool description from help file.

        Returns first paragraph of Description section, cleaned of HTML.
        """
        sections = self._parse_help(tool_id)
        if not sections or "Description" not in sections:
            return None

        desc = sections["Description"]
        # Clean all HTML tags and entities
        desc_clean = self._clean_html_to_text(desc)

        # Get first sentence or ~200 chars
        sentences = desc_clean.split(". ")
        if sentences:
            first_sent = sentences[0].strip()
            if len(first_sent) > 200:
                first_sent = first_sent[:200].rsplit(" ", 1)[0] + "..."
            return first_sent
        return None

    def get_tool_help_excerpt(self, tool_id: str, max_chars: int = 900) -> Optional[str]:
        """Extract a richer plain-text excerpt from the Description section.

        This is used in QGIS `shortHelpString` where HTML rendering is limited.
        """
        sections = self._parse_help(tool_id)
        if not sections or "Description" not in sections:
            return None

        desc = self._clean_html_to_text(sections["Description"])
        if not desc:
            return None

        # Keep up to first 2-3 sentences for readability.
        sentences = re.split(r"(?<=[.!?])\s+", desc)
        excerpt = " ".join(sentences[:3]).strip()
        if len(excerpt) > max_chars:
            excerpt = excerpt[:max_chars].rsplit(" ", 1)[0] + "..."
        return excerpt

    def get_parameter_help(
        self, tool_id: str, param_name: str
    ) -> Optional[str]:
        """Extract help text for specific parameter.

        Looks in Parameters section for the param_name and returns
        the associated description.
        """
        sections = self._parse_help(tool_id)
        if not sections or "Parameters" not in sections:
            return None

        params_text = sections["Parameters"]
        
        # Parameters are typically formatted as:
        #   <p>param_name (type):     description</p>
        # Extract each <p>...</p> block and parse
        
        # Normalize param_name for matching (handle underscores, camelCase variants)
        param_pattern = re.escape(param_name)
        
        # Extract all <p> tags from parameters section
        p_blocks = re.findall(r"<p>(.*?)</p>", params_text, re.DOTALL)
        
        for block in p_blocks:
            # Look for "param_name (type):  description" pattern
            if re.search(rf"\b{param_pattern}\s*\(", block, re.IGNORECASE):
                # Found matching param line
                # Extract everything after the colon
                if ":" in block:
                    desc = block.split(":", 1)[1].strip()
                    # Clean up HTML entities and tags
                    desc = self._clean_html_to_text(desc)
                    desc = desc.strip()

                    if desc:
                        # Limit to ~200 chars with ellipsis
                        if len(desc) > 200:
                            desc = desc[:200].rsplit(" ", 1)[0] + "..."
                        return desc

        return None
    
    def _clean_html_to_text(self, html_text: str) -> str:
        """Convert HTML text to plain text suitable for tooltips.
        
        Removes:
        - All HTML tags
        - Image references
        - HTML entities (converts to text)
        - Excess whitespace
        """
        # Remove image tags entirely (they display as nothing in QGIS)
        text = re.sub(r'<img[^>]*>', '', html_text, flags=re.IGNORECASE)
        
        # Remove HTML tags
        text = re.sub(r'<[^>]+>', '', text)
        
        # Decode HTML entities
        text = html.unescape(text)
        
        # Clean up excessive whitespace
        text = re.sub(r'\s+', ' ', text).strip()
        
        return text

    def _parse_help(self, tool_id: str) -> dict:
        """Parse HTML help file and extract sections."""
        if tool_id in self._cache:
            return self._cache[tool_id]

        if not self.help_dir:
            return {}

        help_file = Path(self.help_dir) / f"{tool_id}.html"
        if not help_file.exists():
            return {}

        try:
            with open(help_file, "r", encoding="utf-8") as f:
                content = f.read()

            # Simple regex-based extraction instead of HTMLParser for now
            # This is more reliable for semi-malformed HTML
            sections = {}

            # Extract Description section
            desc_match = re.search(
                r"<h2>Description</h2>\s*(.*?)(?=<h2>|$)",
                content,
                re.DOTALL | re.IGNORECASE,
            )
            if desc_match:
                sections["Description"] = desc_match.group(1)

            # Extract Parameters section
            params_match = re.search(
                r"<h2>Parameters</h2>\s*(.*?)(?=<h2>|$)",
                content,
                re.DOTALL | re.IGNORECASE,
            )
            if params_match:
                sections["Parameters"] = params_match.group(1)

            # Extract Returns section
            returns_match = re.search(
                r"<h2>Returns</h2>\s*(.*?)(?=<h2>|$)",
                content,
                re.DOTALL | re.IGNORECASE,
            )
            if returns_match:
                sections["Returns"] = returns_match.group(1)

            # Extract References section
            refs_match = re.search(
                r"<h2>References?</h2>\s*(.*?)(?=<h2>|$)",
                content,
                re.DOTALL | re.IGNORECASE,
            )
            if refs_match:
                sections["References"] = refs_match.group(1)

            self._cache[tool_id] = sections
            return sections

        except Exception as e:
            print(f"Error parsing help for {tool_id}: {e}")
            return {}


# Global instance - lazy-loaded on first access
_global_help_provider: Optional[ToolHelpProvider] = None


def get_help_provider() -> ToolHelpProvider:
    """Get global help provider instance."""
    global _global_help_provider
    if _global_help_provider is None:
        _global_help_provider = ToolHelpProvider()
    return _global_help_provider
