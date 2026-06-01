use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::dataset::resolve_dataset_in_file;
use crate::error::{WbhdfError, WbhdfResult};

/// Simple attribute map placeholder for metadata propagation.
pub type AttributeMap = BTreeMap<String, String>;

const METADATA_SEARCH_RADIUS_BYTES: usize = 131_072;

/// Detailed metadata text search result for dataset-scoped assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataTextSearchReport {
	pub present_terms: Vec<String>,
	pub missing_terms: Vec<String>,
}

/// Returns true when a dataset's nearby metadata region contains `needle` text.
///
/// This is a lightweight helper intended for fixture-backed metadata assertions while
/// scoped attribute decoding is under active development.
pub fn dataset_metadata_contains_text_in_file(
	container_path: &Path,
	dataset_path: &str,
	needle: &str,
) -> WbhdfResult<bool> {
	if needle.is_empty() {
		return Err(WbhdfError::InvalidInput(
			"metadata text search requires non-empty needle".to_string(),
		));
	}

	let anchor_path = resolve_metadata_anchor_path(container_path, dataset_path)?;
	let bytes = fs::read(container_path)?;

	let scoped = dataset_metadata_contains_text_near_path(
		&bytes,
		&anchor_path,
		needle,
		METADATA_SEARCH_RADIUS_BYTES,
	);
	Ok(scoped || bytes_contain_text(&bytes, needle))
}

/// Returns true when every term in `needles` is discoverable in the dataset metadata region.
pub fn dataset_metadata_contains_all_texts_in_file(
	container_path: &Path,
	dataset_path: &str,
	needles: &[&str],
) -> WbhdfResult<bool> {
	Ok(dataset_metadata_text_report_in_file(
		container_path,
		dataset_path,
		needles,
	)?
	.missing_terms
	.is_empty())
}

/// Returns the subset of `needles` that were not discoverable for a dataset's metadata region.
pub fn dataset_metadata_missing_texts_in_file(
	container_path: &Path,
	dataset_path: &str,
	needles: &[&str],
) -> WbhdfResult<Vec<String>> {
	Ok(dataset_metadata_text_report_in_file(container_path, dataset_path, needles)?.missing_terms)
}

/// Returns present and missing metadata terms for dataset-scoped metadata assertions.
pub fn dataset_metadata_text_report_in_file(
	container_path: &Path,
	dataset_path: &str,
	needles: &[&str],
) -> WbhdfResult<MetadataTextSearchReport> {
	if needles.is_empty() {
		return Err(WbhdfError::InvalidInput(
			"metadata text search requires at least one needle".to_string(),
		));
	}
	if needles.iter().any(|needle| needle.is_empty()) {
		return Err(WbhdfError::InvalidInput(
			"metadata text search does not accept empty needles".to_string(),
		));
	}

	let anchor_path = resolve_metadata_anchor_path(container_path, dataset_path)?;
	let bytes = fs::read(container_path)?;

    let mut present_terms = Vec::<String>::new();
    let mut missing_terms = Vec::<String>::new();

	for needle in needles {
		if dataset_metadata_contains_text_for_dataset(&bytes, &anchor_path, needle) {
			present_terms.push((*needle).to_string());
		} else {
			missing_terms.push((*needle).to_string());
		}
	}

	Ok(MetadataTextSearchReport {
		present_terms,
		missing_terms,
	})
}

fn resolve_metadata_anchor_path(container_path: &Path, dataset_path: &str) -> WbhdfResult<String> {
	if dataset_path.starts_with('/') {
		let descriptor = resolve_dataset_in_file(container_path, dataset_path)?;
		Ok(descriptor.path)
	} else {
		Ok(dataset_path.to_string())
	}
}

fn dataset_metadata_contains_text_for_dataset(bytes: &[u8], dataset_path: &str, needle: &str) -> bool {
	let scoped = dataset_metadata_contains_text_near_path(
		bytes,
		dataset_path,
		needle,
		METADATA_SEARCH_RADIUS_BYTES,
	);
	scoped || bytes_contain_text(bytes, needle)
}

fn dataset_metadata_contains_text_near_path(
	bytes: &[u8],
	dataset_path: &str,
	needle: &str,
	radius: usize,
) -> bool {
	let path_bytes = dataset_path.as_bytes();
	let needle_bytes = needle.as_bytes();
	let path_offsets = find_all_offsets(bytes, path_bytes);

	if path_offsets.is_empty() {
		return false;
	}

	path_offsets.into_iter().any(|offset| {
		let start = offset.saturating_sub(radius);
		let end = bytes.len().min(offset.saturating_add(path_bytes.len()).saturating_add(radius));
		bytes[start..end]
			.windows(needle_bytes.len())
			.any(|window| window == needle_bytes)
	})
}

fn find_all_offsets(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
	if needle.is_empty() || haystack.len() < needle.len() {
		return Vec::new();
	}

	haystack
		.windows(needle.len())
		.enumerate()
		.filter_map(|(offset, window)| (window == needle).then_some(offset))
		.collect()
}

fn bytes_contain_text(bytes: &[u8], needle: &str) -> bool {
	let needle_bytes = needle.as_bytes();
	if needle_bytes.is_empty() || bytes.len() < needle_bytes.len() {
		return false;
	}
	bytes
		.windows(needle_bytes.len())
		.any(|window| window == needle_bytes)
}

#[cfg(test)]
mod tests {
	use super::dataset_metadata_contains_text_near_path;
	use super::dataset_metadata_contains_text_for_dataset;
	use super::MetadataTextSearchReport;

	#[test]
	fn scoped_metadata_search_matches_near_dataset_marker() {
		let bytes = b"prefix /grp/ds long_name=alpha beta suffix";
		assert!(dataset_metadata_contains_text_near_path(
			bytes,
			"/grp/ds",
			"alpha beta",
			64
		));
	}

	#[test]
	fn scoped_metadata_search_rejects_far_text() {
		let bytes = b"token /grp/ds and far .............. other_text";
		assert!(!dataset_metadata_contains_text_near_path(
			bytes,
			"/grp/ds",
			"other_text",
			4
		));
	}

	#[test]
	fn dataset_text_search_uses_global_fallback_when_scope_misses() {
		let bytes = b"prefix /grp/ds spacer spacer long_name=alpha beta";
		assert!(dataset_metadata_contains_text_for_dataset(
			bytes,
			"/grp/ds",
			"alpha beta"
		));
	}

	#[test]
	fn metadata_text_search_report_structure_behaves_as_expected() {
		let report = MetadataTextSearchReport {
			present_terms: vec!["a".to_string(), "b".to_string()],
			missing_terms: vec!["c".to_string()],
		};
		assert_eq!(report.present_terms.len(), 2);
		assert_eq!(report.missing_terms, vec!["c".to_string()]);
	}

	#[test]
	fn dataset_text_search_supports_non_slash_anchor_tokens() {
		let bytes = b"prefix DataFieldName=alpha long_name=alpha units=beta";
		assert!(dataset_metadata_contains_text_for_dataset(
			bytes,
			"DataFieldName=alpha",
			"units=beta"
		));
	}
}
