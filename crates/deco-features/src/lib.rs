use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use deco_config::{DevcontainerConfigKind, ResolvedReadConfiguration};
use deco_core_model::{DecoError, ErrorCategory};
use deco_lockfile::{FeatureLockfileDocument, FeatureLockfileEntry};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeaturesSource {
    ManifestDirectory,
    DevcontainerConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureManifestSummary {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installs_after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureReferenceSummary {
    pub reference: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installs_after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeaturesResult {
    pub source: FeaturesSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manifests: Vec<FeatureManifestSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<FeatureReferenceSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureDependencyNode {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installs_after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureDependencyResolutionResult {
    pub source: FeaturesSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<FeatureDependencyNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureTestFailure {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureTestResult {
    pub manifest_dir: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<FeatureTestFailure>,
}

pub fn discover_feature_manifests(
    directory: impl AsRef<Path>,
) -> Result<Vec<FeatureManifestSummary>, DecoError> {
    let directory = directory.as_ref();
    if !directory.exists() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("feature manifest directory `{}` does not exist", directory.display()),
        ));
    }
    if !directory.is_dir() {
        return Err(DecoError::new(
            ErrorCategory::Config,
            format!("feature manifest directory `{}` is not a directory", directory.display()),
        ));
    }

    let mut manifests = Vec::new();
    for path in collect_json_files(directory)? {
        manifests.push(parse_feature_manifest_summary(&path)?);
    }

    manifests.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(manifests)
}

pub fn resolve_feature_dependencies(
    manifest_dir: Option<&Path>,
    resolved: Option<&ResolvedReadConfiguration>,
) -> Result<FeatureDependencyResolutionResult, DecoError> {
    if let Some(manifest_dir) = manifest_dir {
        let manifests = discover_feature_manifests(manifest_dir)?;
        let mut nodes: Vec<FeatureDependencyNode> = manifests
            .into_iter()
            .map(|manifest| FeatureDependencyNode {
                id: manifest.id.unwrap_or_else(|| manifest.path.clone()),
                path: Some(manifest.path),
                depends_on: manifest.depends_on,
                installs_after: manifest.installs_after,
            })
            .collect();
        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        let roots = nodes
            .iter()
            .filter(|node| node.depends_on.is_empty() && node.installs_after.is_empty())
            .map(|node| node.id.clone())
            .collect();
        return Ok(FeatureDependencyResolutionResult {
            source: FeaturesSource::ManifestDirectory,
            manifest_dir: Some(manifest_dir.display().to_string()),
            workspace_folder: None,
            config_file: None,
            roots,
            nodes,
        });
    }

    let resolved = resolved.ok_or_else(|| {
        DecoError::new(
            ErrorCategory::Internal,
            "either a manifest directory or resolved config is required",
        )
    })?;
    let references = extract_feature_references_from_resolved_config(resolved)?;
    let mut nodes: Vec<FeatureDependencyNode> = references
        .iter()
        .map(|reference| FeatureDependencyNode {
            id: local_feature_manifest_id(reference).unwrap_or_else(|| reference.reference.clone()),
            path: local_feature_manifest_path(reference),
            depends_on: reference.depends_on.clone(),
            installs_after: reference.installs_after.clone(),
        })
        .collect();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let roots = nodes
        .iter()
        .filter(|node| node.depends_on.is_empty() && node.installs_after.is_empty())
        .map(|node| node.id.clone())
        .collect();

    Ok(FeatureDependencyResolutionResult {
        source: FeaturesSource::DevcontainerConfig,
        manifest_dir: None,
        workspace_folder: Some(resolved.workspace_folder.clone()),
        config_file: Some(resolved.config_file.clone()),
        roots,
        nodes,
    })
}

pub fn test_feature_manifests(
    manifest_dir: impl AsRef<Path>,
) -> Result<FeatureTestResult, DecoError> {
    let manifest_dir = manifest_dir.as_ref();
    let manifest_paths = collect_json_files(manifest_dir)?;
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut manifests = Vec::new();
    for path in &manifest_paths {
        match parse_feature_manifest_summary(&path) {
            Ok(manifest) => manifests.push(manifest),
            Err(error) => {
                failures.entry(path.display().to_string()).or_default().push(error.to_string());
            }
        }
    }

    let mut ids_to_paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for manifest in &manifests {
        match manifest.id.as_deref() {
            Some(id) if !id.trim().is_empty() => {
                ids_to_paths.entry(id.to_string()).or_default().push(manifest.path.clone());
            }
            _ => {
                failures
                    .entry(manifest.path.clone())
                    .or_default()
                    .push("feature manifest is missing `id`".to_string());
            }
        }
    }

    for (id, paths) in &ids_to_paths {
        if paths.len() > 1 {
            for path in paths {
                failures
                    .entry(path.clone())
                    .or_default()
                    .push(format!("duplicate feature manifest id `{}`", id));
            }
        }
    }

    let known_ids: BTreeSet<String> = ids_to_paths
        .iter()
        .filter(|(_, paths)| paths.len() == 1)
        .map(|(id, _)| id.clone())
        .collect();
    let known_paths: BTreeSet<String> =
        manifests.iter().map(|manifest| manifest.path.clone()).collect();

    let mut dependency_graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for manifest in &manifests {
        let Some(id) = manifest.id.as_deref().filter(|id| !id.trim().is_empty()) else {
            continue;
        };

        let deps = manifest.depends_on.iter().chain(manifest.installs_after.iter());
        for dep in deps {
            if !known_ids.contains(dep) && !known_paths.contains(dep) {
                failures
                    .entry(manifest.path.clone())
                    .or_default()
                    .push(format!("unresolved dependency `{}`", dep));
                continue;
            }

            dependency_graph.entry(id.to_string()).or_default().insert(dep.clone());
        }
    }

    if failures.is_empty() {
        let cycle_nodes = dependency_cycle_nodes(&dependency_graph);
        for node in cycle_nodes {
            if let Some(paths) = ids_to_paths.get(&node) {
                for path in paths {
                    failures
                        .entry(path.clone())
                        .or_default()
                        .push(format!("dependency cycle detected at `{}`", node));
                }
            }
        }
    }

    let total = manifest_paths.len();
    let failed = failures.len();
    Ok(FeatureTestResult {
        manifest_dir: manifest_dir.display().to_string(),
        total,
        passed: total.saturating_sub(failed),
        failed,
        failures: failures
            .into_iter()
            .map(|(path, messages)| FeatureTestFailure { path, message: messages.join("; ") })
            .collect(),
    })
}

pub fn extract_feature_references(
    configuration: &Value,
    _kind: DevcontainerConfigKind,
) -> Result<Vec<FeatureReferenceSummary>, DecoError> {
    let features = configuration.get("features").and_then(Value::as_object);
    let mut references = Vec::new();

    if let Some(features) = features {
        for (reference, options) in features {
            references.push(FeatureReferenceSummary {
                reference: reference.clone(),
                option_keys: option_keys(options),
                options: Some(options.clone()),
                depends_on: string_list_field(options.get("dependsOn")),
                installs_after: string_list_field(options.get("installsAfter")),
            });
        }
    }

    references.sort_by(|left, right| left.reference.cmp(&right.reference));
    Ok(references)
}

pub fn features_from_read_configuration(
    resolved: &ResolvedReadConfiguration,
) -> Result<FeaturesResult, DecoError> {
    let references = extract_feature_references_from_resolved_config(resolved)?;
    Ok(FeaturesResult {
        source: FeaturesSource::DevcontainerConfig,
        manifest_dir: None,
        workspace_folder: Some(resolved.workspace_folder.clone()),
        config_file: Some(resolved.config_file.clone()),
        manifests: Vec::new(),
        references,
    })
}

pub fn generate_feature_lockfile(
    resolved: &ResolvedReadConfiguration,
) -> Result<FeatureLockfileDocument, DecoError> {
    let references = extract_feature_references_from_resolved_config(resolved)?;
    let mut features = BTreeMap::new();
    for reference in references {
        let id = local_feature_manifest_id(&reference).unwrap_or_else(|| reference.reference.clone());
        let version = local_feature_manifest_version(&reference)
            .or_else(|| version_from_feature_reference(&reference.reference))
            .unwrap_or_else(|| "0.0.0".to_string());
        let integrity = integrity_from_feature_reference(&reference.reference)
            .unwrap_or_else(|| "unresolved".to_string());
        let depends_on = (!reference.depends_on.is_empty()).then_some(reference.depends_on.clone());
        features.insert(
            id,
            FeatureLockfileEntry {
                version,
                resolved: reference.reference.clone(),
                integrity,
                depends_on,
            },
        );
    }

    Ok(FeatureLockfileDocument { features })
}

fn extract_feature_references_from_resolved_config(
    resolved: &ResolvedReadConfiguration,
) -> Result<Vec<FeatureReferenceSummary>, DecoError> {
    let mut references = extract_feature_references(&resolved.configuration, resolved.kind)?;
    let config_dir = Path::new(&resolved.config_file).parent().ok_or_else(|| {
        DecoError::new(ErrorCategory::Internal, "config file has no parent directory")
    })?;

    for reference in &mut references {
            if let Some(manifest) = resolve_local_feature_manifest(config_dir, &reference.reference)? {
            if reference.depends_on.is_empty() {
                reference.depends_on = manifest.depends_on;
            }
            if reference.installs_after.is_empty() {
                reference.installs_after = manifest.installs_after;
            }
            reference.options.get_or_insert_with(|| Value::Object(Default::default()));
                if let Some(options) = reference.options.as_mut().and_then(Value::as_object_mut) {
                    options.insert(
                        "__deco_local_feature_id".to_string(),
                        Value::String(manifest.id.unwrap_or_default()),
                    );
                    if let Some(version) = manifest.version {
                        options.insert(
                            "__deco_local_feature_version".to_string(),
                            Value::String(version),
                        );
                    }
                    options
                        .insert("__deco_local_feature_path".to_string(), Value::String(manifest.path));
                }
            }
    }

    Ok(references)
}

fn collect_json_files(directory: &Path) -> Result<Vec<PathBuf>, DecoError> {
    let mut files = Vec::new();
    let mut stack = vec![directory.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|error| {
            DecoError::new(
                ErrorCategory::Config,
                format!("failed to read feature manifest directory `{}`", dir.display()),
            )
            .with_details(error.to_string())
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                DecoError::new(
                    ErrorCategory::Config,
                    format!("failed to read feature manifest directory `{}`", dir.display()),
                )
                .with_details(error.to_string())
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(|value| value.to_string())
}

fn option_names(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_object)
        .map(|object| {
            let mut names = BTreeSet::new();
            for key in object.keys() {
                names.insert(key.clone());
            }
            names.into_iter().collect()
        })
        .unwrap_or_default()
}

fn option_keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .map(|object| {
            let mut keys = BTreeSet::new();
            for key in object.keys() {
                if key != "dependsOn" && key != "installsAfter" {
                    keys.insert(key.clone());
                }
            }
            keys.into_iter().collect()
        })
        .unwrap_or_default()
}

fn string_list_field(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(ToOwned::to_owned))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        Some(Value::Object(items)) => {
            items.keys().cloned().collect::<BTreeSet<_>>().into_iter().collect()
        }
        _ => Vec::new(),
    }
}

fn parse_feature_manifest_summary(path: &Path) -> Result<FeatureManifestSummary, DecoError> {
    let content = fs::read_to_string(path).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to read feature manifest `{}`", path.display()),
        )
        .with_details(error.to_string())
    })?;

    let manifest: Value = serde_json::from_str(&content).map_err(|error| {
        DecoError::new(
            ErrorCategory::Config,
            format!("failed to parse feature manifest `{}`", path.display()),
        )
        .with_details(error.to_string())
    })?;

    let object = manifest.as_object().ok_or_else(|| {
        DecoError::new(
            ErrorCategory::Config,
            format!("feature manifest `{}` must be a JSON object", path.display()),
        )
    })?;

    Ok(FeatureManifestSummary {
        path: path.display().to_string(),
        id: string_field(object, "id"),
        version: string_field(object, "version"),
        name: string_field(object, "name"),
        description: string_field(object, "description"),
        option_names: option_names(object.get("options")),
        depends_on: string_list_field(object.get("dependsOn")),
        installs_after: string_list_field(object.get("installsAfter")),
    })
}

fn resolve_local_feature_manifest(
    config_dir: &Path,
    reference: &str,
) -> Result<Option<FeatureManifestSummary>, DecoError> {
    if !(reference.starts_with("./") || reference.starts_with("../")) {
        return Ok(None);
    }

    let base_path = config_dir.join(reference);
    let manifest_path =
        if base_path.is_dir() { base_path.join("devcontainer-feature.json") } else { base_path };

    if !manifest_path.exists() {
        return Ok(None);
    }

    parse_feature_manifest_summary(&manifest_path).map(Some)
}

fn local_feature_manifest_id(reference: &FeatureReferenceSummary) -> Option<String> {
    reference.options.as_ref().and_then(|options| {
        options
            .get("__deco_local_feature_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn local_feature_manifest_path(reference: &FeatureReferenceSummary) -> Option<String> {
    reference.options.as_ref().and_then(|options| {
        options.get("__deco_local_feature_path").and_then(Value::as_str).map(ToOwned::to_owned)
    })
}

fn local_feature_manifest_version(reference: &FeatureReferenceSummary) -> Option<String> {
    reference.options.as_ref().and_then(|options| {
        options
            .get("__deco_local_feature_version")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn version_from_feature_reference(reference: &str) -> Option<String> {
    let (_, suffix) = reference.rsplit_once([':', '@'])?;
    if suffix.is_empty() || suffix.contains('/') {
        return None;
    }
    Some(suffix.to_string())
}

fn integrity_from_feature_reference(reference: &str) -> Option<String> {
    reference
        .split_once("@sha256:")
        .map(|(_, digest)| format!("sha256:{digest}"))
}

fn dependency_cycle_nodes(graph: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Unvisited,
        Visiting,
        Visited,
    }

    fn visit(
        node: &str,
        graph: &BTreeMap<String, BTreeSet<String>>,
        states: &mut BTreeMap<String, State>,
        stack: &mut Vec<String>,
        cycle_nodes: &mut BTreeSet<String>,
    ) {
        match states.get(node).copied().unwrap_or(State::Unvisited) {
            State::Visiting => {
                if let Some(position) = stack.iter().position(|value| value == node) {
                    for value in &stack[position..] {
                        cycle_nodes.insert(value.clone());
                    }
                }
                return;
            }
            State::Visited => return,
            State::Unvisited => {}
        }

        states.insert(node.to_string(), State::Visiting);
        stack.push(node.to_string());

        if let Some(neighbours) = graph.get(node) {
            for neighbour in neighbours {
                visit(neighbour, graph, states, stack, cycle_nodes);
            }
        }

        stack.pop();
        states.insert(node.to_string(), State::Visited);
    }

    let mut states: BTreeMap<String, State> = BTreeMap::new();
    let mut stack = Vec::new();
    let mut cycle_nodes = BTreeSet::new();

    for node in graph.keys() {
        visit(node, graph, &mut states, &mut stack, &mut cycle_nodes);
    }

    cycle_nodes
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use deco_config::{DevcontainerConfigKind, ResolvedReadConfiguration};
    use serde_json::json;

    use super::*;

    fn tempdir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "deco-features-test-{}-{}",
            std::process::id(),
            suffix
        ));
        fs::create_dir_all(&path).expect("tempdir should be created");
        path
    }

    #[test]
    fn discovers_feature_manifests_from_directory_tree() {
        let temp = tempdir();
        let nested = temp.join("nested");
        fs::create_dir_all(&nested).expect("nested dir should exist");
        fs::write(
            temp.join("base.json"),
            r#"{
              "id": "base",
              "version": "1.0.0",
              "name": "Base",
              "options": {
                "foo": {},
                "bar": {}
              }
            }"#,
        )
        .expect("manifest should be written");
        fs::write(
            nested.join("extra.json"),
            r#"{
              "id": "extra",
              "version": "2.0.0",
              "description": "Extra"
            }"#,
        )
        .expect("manifest should be written");

        let manifests = discover_feature_manifests(&temp).expect("manifests should be discovered");

        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id.as_deref(), Some("base"));
        assert_eq!(manifests[0].option_names, vec!["bar".to_string(), "foo".to_string()]);
        assert_eq!(manifests[1].id.as_deref(), Some("extra"));
    }

    #[test]
    fn extracts_feature_references_from_config() {
        let configuration = json!({
            "features": {
                "ghcr.io/devcontainers/features/common-utils:2": {
                    "installZsh": true,
                    "username": "vscode",
                    "dependsOn": ["ghcr.io/devcontainers/features/git:1"],
                    "installsAfter": ["ghcr.io/devcontainers/features/node:1"]
                }
            }
        });

        let references = extract_feature_references(&configuration, DevcontainerConfigKind::Image)
            .expect("references should be extracted");

        assert_eq!(references.len(), 1);
        assert_eq!(references[0].reference, "ghcr.io/devcontainers/features/common-utils:2");
        assert_eq!(
            references[0].option_keys,
            vec!["installZsh".to_string(), "username".to_string()]
        );
        assert_eq!(
            references[0].depends_on,
            vec!["ghcr.io/devcontainers/features/git:1".to_string()]
        );
        assert_eq!(
            references[0].installs_after,
            vec!["ghcr.io/devcontainers/features/node:1".to_string()]
        );
    }

    #[test]
    fn resolves_dependencies_from_config_metadata() {
        let configuration = json!({
            "features": {
                "feature-a": {
                    "dependsOn": ["feature-b"],
                    "installsAfter": ["feature-c"]
                },
                "feature-b": {}
            }
        });
        let resolved = ResolvedReadConfiguration {
            workspace_folder: "/workspace".to_string(),
            config_file: "/workspace/.devcontainer/devcontainer.json".to_string(),
            kind: DevcontainerConfigKind::Image,
            normalized: deco_config::NormalizedDevcontainerConfig {
                name: None,
                image: None,
                build: None,
                compose: None,
                workspace_folder: None,
                remote_user: None,
                remote_env: None,
            },
            configuration,
            merged_configuration: None,
        };

        let result = resolve_feature_dependencies(None, Some(&resolved))
            .expect("dependency resolution should succeed");

        assert_eq!(result.source, FeaturesSource::DevcontainerConfig);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.nodes[0].id, "feature-a");
        assert_eq!(result.nodes[0].depends_on, vec!["feature-b".to_string()]);
        assert_eq!(result.nodes[0].installs_after, vec!["feature-c".to_string()]);
        assert_eq!(result.roots, vec!["feature-b".to_string()]);
    }

    #[test]
    fn features_from_read_configuration_wraps_config_metadata() {
        let resolved = ResolvedReadConfiguration {
            workspace_folder: "/workspace".to_string(),
            config_file: "/workspace/.devcontainer/devcontainer.json".to_string(),
            kind: DevcontainerConfigKind::Image,
            normalized: deco_config::NormalizedDevcontainerConfig {
                name: None,
                image: None,
                build: None,
                compose: None,
                workspace_folder: None,
                remote_user: None,
                remote_env: None,
            },
            configuration: json!({"features": {}}),
            merged_configuration: None,
        };

        let result =
            features_from_read_configuration(&resolved).expect("features result should exist");

        assert_eq!(result.source, FeaturesSource::DevcontainerConfig);
        assert_eq!(result.workspace_folder.as_deref(), Some("/workspace"));
        assert_eq!(
            result.config_file.as_deref(),
            Some("/workspace/.devcontainer/devcontainer.json")
        );
    }

    #[test]
    fn enriches_local_feature_references_from_config_dir() {
        let temp = tempdir();
        let config_dir = temp.join(".devcontainer");
        let feature_dir = config_dir.join("features").join("feature-a");
        fs::create_dir_all(&feature_dir).expect("feature dir should exist");
        fs::write(
            feature_dir.join("devcontainer-feature.json"),
            r#"{
              "id": "feature-a",
              "dependsOn": ["feature-b"]
            }"#,
        )
        .expect("manifest should be written");

        let resolved = ResolvedReadConfiguration {
            workspace_folder: temp.display().to_string(),
            config_file: config_dir.join("devcontainer.json").display().to_string(),
            kind: DevcontainerConfigKind::Image,
            normalized: deco_config::NormalizedDevcontainerConfig {
                name: None,
                image: None,
                build: None,
                compose: None,
                workspace_folder: None,
                remote_user: None,
                remote_env: None,
            },
            configuration: json!({
                "features": {
                    "./features/feature-a": {}
                }
            }),
            merged_configuration: None,
        };

        let references = extract_feature_references_from_resolved_config(&resolved)
            .expect("references should be enriched");
        let result = resolve_feature_dependencies(None, Some(&resolved))
            .expect("dependency resolution should succeed");

        assert_eq!(references.len(), 1);
        assert_eq!(references[0].depends_on, vec!["feature-b".to_string()]);
        assert_eq!(result.nodes[0].id, "feature-a");
        assert_eq!(result.nodes[0].depends_on, vec!["feature-b".to_string()]);
        assert!(
            result.nodes[0]
                .path
                .as_deref()
                .is_some_and(|path| path.ends_with("devcontainer-feature.json"))
        );
    }

    #[test]
    fn test_feature_manifests_reports_validation_failures() {
        let temp = tempdir();
        fs::write(
            temp.join("base.json"),
            r#"{
              "id": "base",
              "dependsOn": ["missing"]
            }"#,
        )
        .expect("manifest should be written");
        fs::write(temp.join("broken.json"), r#"{ "name": "Broken" }"#)
            .expect("manifest should be written");

        let result = test_feature_manifests(&temp).expect("feature test should complete");

        assert_eq!(result.total, 2);
        assert_eq!(result.failed, 2);
        assert_eq!(result.passed, 0);
        assert_eq!(result.failures.len(), 2);
    }
}
