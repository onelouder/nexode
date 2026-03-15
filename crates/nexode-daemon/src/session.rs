use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use nexode_proto::AgentMode;
use serde::Deserialize;
use serde::de::Error as _;
use serde_yaml::Value;
use thiserror::Error;

const DEFAULT_VERSION: &str = "2.0";
const DEFAULT_MODEL: &str = "claude-code";
const DEFAULT_TIMEOUT_MINUTES: u64 = 120;
const DEFAULT_SESSION_NAME: &str = "default";

#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    pub version: String,
    pub session: SessionMetadata,
    pub defaults: EffectiveDefaults,
    pub models: BTreeMap<String, ModelPricing>,
    pub projects: Vec<ProjectConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionMetadata {
    pub name: String,
    pub budget: BudgetConfig,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BudgetConfig {
    pub max_usd: Option<f64>,
    pub warn_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    pub input_per_1m: f64,
    pub output_per_1m: f64,
    pub cache_read_per_1m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfig {
    pub id: String,
    pub repo: Option<PathBuf>,
    pub display_name: String,
    pub color: Option<String>,
    pub tags: Vec<String>,
    pub budget: BudgetConfig,
    pub verify: Option<VerifyConfig>,
    pub defaults: EffectiveDefaults,
    pub slots: Vec<SlotConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyConfig {
    pub build: Option<String>,
    pub test: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotConfig {
    pub id: String,
    pub task: String,
    pub model: String,
    pub mode: AgentMode,
    pub branch: String,
    pub timeout_minutes: u64,
    pub provider_config: BTreeMap<String, String>,
    pub context: ContextConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveDefaults {
    pub model: String,
    pub mode: AgentMode,
    pub timeout_minutes: u64,
    pub provider_config: BTreeMap<String, String>,
    pub context: ContextConfig,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContextConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SessionConfigError {
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse yaml in `{path}`: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("include cycle detected at `{0}`")]
    IncludeCycle(PathBuf),
    #[error("{0}")]
    Validation(String),
}

pub fn load_session_config(path: impl AsRef<Path>) -> Result<SessionConfig, SessionConfigError> {
    let include_path = canonicalize_existing(path.as_ref())?;
    let mut include_stack = BTreeSet::from([include_path.clone()]);
    let root = load_root_file(&include_path, &mut include_stack)?;

    let session_defaults = resolve_defaults(None, root.defaults.as_ref())?;
    let session_budget = resolve_budget(root.session.budget.as_ref(), "session budget")?;

    let mut seen_projects = BTreeSet::new();
    let mut projects = Vec::with_capacity(root.projects.len());

    for located in root.projects {
        if !seen_projects.insert(located.project.id.clone()) {
            return Err(SessionConfigError::Validation(format!(
                "duplicate project id `{}`",
                located.project.id
            )));
        }

        validate_identifier(&located.project.id, "project")?;

        let repo = located
            .project
            .repo
            .as_deref()
            .map(|value| resolve_path(value, &located.base_dir))
            .transpose()?;

        let repo_local = load_repo_local_config(repo.as_deref())?;
        let project_defaults =
            resolve_defaults(Some(&session_defaults), located.project.defaults.as_ref())?;
        let merged_project_defaults =
            resolve_defaults(Some(&project_defaults), repo_local.defaults.as_ref())?;

        let tags = merge_string_array(&[located.project.tags.as_ref(), repo_local.tags.as_ref()]);
        let budget = merge_budget(located.project.budget.as_ref(), repo_local.budget.as_ref());
        let verify = merge_verify(located.project.verify.as_ref(), repo_local.verify.as_ref());
        let display_name = repo_local
            .display_name
            .clone()
            .or_else(|| located.project.display_name.clone())
            .unwrap_or_else(|| located.project.id.clone());
        let color = repo_local
            .color
            .clone()
            .or_else(|| located.project.color.clone());
        let slots = resolve_slots(
            &located.project,
            &repo_local,
            &merged_project_defaults,
            &located.project.id,
        )?;

        projects.push(ProjectConfig {
            id: located.project.id,
            repo,
            display_name,
            color,
            tags,
            budget: resolve_budget(Some(&budget), "project budget")?,
            verify,
            defaults: merged_project_defaults,
            slots,
        });
    }

    Ok(SessionConfig {
        version: root.version,
        session: SessionMetadata {
            name: root.session.name,
            budget: session_budget,
        },
        defaults: session_defaults,
        models: root
            .models
            .into_iter()
            .map(|(name, raw)| {
                (
                    name,
                    ModelPricing {
                        input_per_1m: raw.input_per_1m,
                        output_per_1m: raw.output_per_1m,
                        cache_read_per_1m: raw.cache_read_per_1m,
                    },
                )
            })
            .collect(),
        projects,
    })
}

fn resolve_slots(
    project: &ProjectRaw,
    repo_local: &RepoLocalRaw,
    project_defaults: &EffectiveDefaults,
    project_id: &str,
) -> Result<Vec<SlotConfig>, SessionConfigError> {
    let mut ordered_ids = Vec::new();
    let mut session_slots = BTreeMap::new();
    let mut repo_slots = BTreeMap::new();

    for slot in project.slots.iter().flatten() {
        validate_identifier(&slot.id, "slot")?;
        if session_slots
            .insert(slot.id.clone(), slot.clone())
            .is_some()
        {
            return Err(SessionConfigError::Validation(format!(
                "duplicate slot id `{}` in project `{project_id}`",
                slot.id
            )));
        }
        ordered_ids.push(slot.id.clone());
    }

    for slot in repo_local.slots.iter().flatten() {
        validate_identifier(&slot.id, "slot")?;
        if repo_slots.insert(slot.id.clone(), slot.clone()).is_some() {
            return Err(SessionConfigError::Validation(format!(
                "duplicate repo-local slot id `{}` in project `{project_id}`",
                slot.id
            )));
        }
        if !session_slots.contains_key(&slot.id) {
            ordered_ids.push(slot.id.clone());
        }
    }

    let mut resolved = Vec::with_capacity(ordered_ids.len());

    for slot_id in ordered_ids {
        let merged = merge_slot(
            session_slots.get(&slot_id),
            repo_slots.get(&slot_id),
            project_id,
        )?;
        let defaults = apply_defaults_to_slot(project_defaults, &merged)?;

        resolved.push(SlotConfig {
            branch: merged
                .branch
                .clone()
                .unwrap_or_else(|| format!("agent/{slot_id}")),
            id: slot_id,
            task: merged.task,
            model: defaults.model,
            mode: defaults.mode,
            timeout_minutes: defaults.timeout_minutes,
            provider_config: defaults.provider_config,
            context: defaults.context,
        });
    }

    Ok(resolved)
}

fn merge_slot(
    session_slot: Option<&SlotRaw>,
    repo_slot: Option<&SlotRaw>,
    project_id: &str,
) -> Result<SlotRaw, SessionConfigError> {
    let base = session_slot.or(repo_slot).ok_or_else(|| {
        SessionConfigError::Validation(format!("project `{project_id}` has an empty slot entry"))
    })?;
    let overlay = repo_slot.filter(|_| session_slot.is_some());

    Ok(SlotRaw {
        id: base.id.clone(),
        task: overlay
            .and_then(|slot| Some(slot.task.clone()))
            .unwrap_or_else(|| base.task.clone()),
        model: overlay
            .and_then(|slot| slot.model.clone())
            .or_else(|| base.model.clone()),
        mode: overlay.and_then(|slot| slot.mode).or(base.mode),
        branch: overlay
            .and_then(|slot| slot.branch.clone())
            .or_else(|| base.branch.clone()),
        timeout_minutes: overlay
            .and_then(|slot| slot.timeout_minutes)
            .or(base.timeout_minutes),
        provider_config: merge_map_option(
            base.provider_config.as_ref(),
            overlay.and_then(|slot| slot.provider_config.as_ref()),
        ),
        context: merge_context_option(
            base.context.as_ref(),
            overlay.and_then(|slot| slot.context.as_ref()),
        ),
    })
}

fn apply_defaults_to_slot(
    defaults: &EffectiveDefaults,
    slot: &SlotRaw,
) -> Result<EffectiveDefaults, SessionConfigError> {
    let mode = slot.mode.map(YamlMode::into_proto).unwrap_or(defaults.mode);

    Ok(EffectiveDefaults {
        model: slot.model.clone().unwrap_or_else(|| defaults.model.clone()),
        mode,
        timeout_minutes: slot.timeout_minutes.unwrap_or(defaults.timeout_minutes),
        provider_config: merge_maps(
            Some(&defaults.provider_config),
            slot.provider_config.as_ref(),
        ),
        context: merge_contexts(Some(&defaults.context), slot.context.as_ref()),
    })
}

fn resolve_defaults(
    parent: Option<&EffectiveDefaults>,
    overlay: Option<&DefaultsRaw>,
) -> Result<EffectiveDefaults, SessionConfigError> {
    let base = parent.cloned().unwrap_or_else(|| EffectiveDefaults {
        model: DEFAULT_MODEL.to_string(),
        mode: AgentMode::Plan,
        timeout_minutes: DEFAULT_TIMEOUT_MINUTES,
        provider_config: BTreeMap::new(),
        context: ContextConfig::default(),
    });

    Ok(EffectiveDefaults {
        model: overlay
            .and_then(|layer| layer.model.clone())
            .unwrap_or(base.model),
        mode: overlay
            .and_then(|layer| layer.mode)
            .map(YamlMode::into_proto)
            .unwrap_or(base.mode),
        timeout_minutes: overlay
            .and_then(|layer| layer.timeout_minutes)
            .unwrap_or(base.timeout_minutes),
        provider_config: merge_maps(
            Some(&base.provider_config),
            overlay.and_then(|d| d.provider_config.as_ref()),
        ),
        context: merge_contexts(
            Some(&base.context),
            overlay.and_then(|d| d.context.as_ref()),
        ),
    })
}

fn merge_contexts(base: Option<&ContextConfig>, overlay: Option<&ContextRaw>) -> ContextConfig {
    let include = merge_optional_array(
        base.and_then(|ctx| ctx.include.as_ref()),
        overlay.and_then(|ctx| ctx.include.as_ref()),
    );
    let exclude = merge_string_array(&[
        base.map(|ctx| &ctx.exclude),
        overlay.and_then(|ctx| ctx.exclude.as_ref()),
    ]);

    ContextConfig { include, exclude }
}

fn merge_context_option(
    base: Option<&ContextRaw>,
    overlay: Option<&ContextRaw>,
) -> Option<ContextRaw> {
    if base.is_none() && overlay.is_none() {
        return None;
    }

    Some(ContextRaw {
        include: merge_optional_array(
            base.and_then(|ctx| ctx.include.as_ref()),
            overlay.and_then(|ctx| ctx.include.as_ref()),
        ),
        exclude: Some(merge_string_array(&[
            base.and_then(|ctx| ctx.exclude.as_ref()),
            overlay.and_then(|ctx| ctx.exclude.as_ref()),
        ])),
    })
}

fn merge_optional_array(
    base: Option<&Vec<String>>,
    overlay: Option<&Vec<String>>,
) -> Option<Vec<String>> {
    let mut has_value = false;
    let mut merged = Vec::new();
    let mut seen = BTreeSet::new();

    if let Some(values) = base {
        has_value = true;
        append_unique(&mut merged, &mut seen, values);
    }

    if let Some(values) = overlay {
        has_value = true;
        if values.is_empty() {
            merged.clear();
            seen.clear();
        } else {
            append_unique(&mut merged, &mut seen, values);
        }
    }

    has_value.then_some(merged)
}

fn merge_string_array(layers: &[Option<&Vec<String>>]) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = BTreeSet::new();

    for values in layers.iter().flatten() {
        if values.is_empty() {
            merged.clear();
            seen.clear();
            continue;
        }

        append_unique(&mut merged, &mut seen, values);
    }

    merged
}

fn append_unique(target: &mut Vec<String>, seen: &mut BTreeSet<String>, values: &[String]) {
    for value in values {
        if seen.insert(value.clone()) {
            target.push(value.clone());
        }
    }
}

fn merge_maps(
    base: Option<&BTreeMap<String, String>>,
    overlay: Option<&BTreeMap<String, String>>,
) -> BTreeMap<String, String> {
    let mut merged = BTreeMap::new();

    if let Some(base) = base {
        merged.extend(base.clone());
    }

    if let Some(overlay) = overlay {
        merged.extend(overlay.clone());
    }

    merged
}

fn merge_map_option(
    base: Option<&BTreeMap<String, String>>,
    overlay: Option<&BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    if base.is_none() && overlay.is_none() {
        None
    } else {
        Some(merge_maps(base, overlay))
    }
}

fn merge_budget(session: Option<&BudgetRaw>, repo: Option<&BudgetRaw>) -> BudgetRaw {
    BudgetRaw {
        max_usd: repo
            .and_then(|raw| raw.max_usd)
            .or(session.and_then(|raw| raw.max_usd)),
        warn_usd: repo
            .and_then(|raw| raw.warn_usd)
            .or(session.and_then(|raw| raw.warn_usd)),
    }
}

fn merge_verify(session: Option<&VerifyRaw>, repo: Option<&VerifyRaw>) -> Option<VerifyConfig> {
    let build = repo
        .and_then(|raw| raw.build.clone())
        .or_else(|| session.and_then(|raw| raw.build.clone()));
    let test = repo
        .and_then(|raw| raw.test.clone())
        .or_else(|| session.and_then(|raw| raw.test.clone()));

    if build.is_none() && test.is_none() {
        None
    } else {
        Some(VerifyConfig { build, test })
    }
}

fn resolve_budget(
    raw: Option<&BudgetRaw>,
    label: &str,
) -> Result<BudgetConfig, SessionConfigError> {
    let max_usd = raw.and_then(|raw| raw.max_usd);
    let warn_usd = raw
        .and_then(|raw| raw.warn_usd)
        .or_else(|| max_usd.map(|value| value * 0.8));

    if let (Some(warn), Some(max)) = (warn_usd, max_usd) {
        if warn > max {
            return Err(SessionConfigError::Validation(format!(
                "{label} warn_usd ({warn}) cannot exceed max_usd ({max})"
            )));
        }
    }

    Ok(BudgetConfig { max_usd, warn_usd })
}

fn load_root_file(
    path: &Path,
    include_stack: &mut BTreeSet<PathBuf>,
) -> Result<RootFile, SessionConfigError> {
    let value = load_yaml_value(path)?;

    if matches!(
        lookup_mapping_key(&value, "projects"),
        Some(Value::Sequence(_))
    ) {
        let raw: RootV2Raw = deserialize_value(path, value)?;
        let projects = expand_project_entries(&raw.projects, parent_dir(path), include_stack)?;

        Ok(RootFile {
            version: raw.version,
            session: raw.session,
            defaults: raw.defaults,
            models: raw.models.unwrap_or_default(),
            projects,
        })
    } else {
        let raw: RootLegacyRaw = deserialize_value(path, value)?;
        let project_id = raw.id.unwrap_or_else(|| "default".to_string());

        Ok(RootFile {
            version: raw.version.unwrap_or_else(|| DEFAULT_VERSION.to_string()),
            session: raw.session.unwrap_or_else(|| SessionMetadataRaw {
                name: DEFAULT_SESSION_NAME.to_string(),
                budget: None,
            }),
            defaults: raw.defaults,
            models: raw.models.unwrap_or_default(),
            projects: vec![LocatedProject {
                base_dir: parent_dir(path),
                project: ProjectRaw {
                    id: project_id.clone(),
                    repo: raw.repo,
                    display_name: raw.display_name.or(Some(project_id.clone())),
                    color: raw.color,
                    tags: raw.tags,
                    budget: raw.budget,
                    defaults: None,
                    verify: raw.verify,
                    slots: raw.slots,
                },
            }],
        })
    }
}

fn expand_project_entries(
    entries: &[ProjectEntryRaw],
    base_dir: PathBuf,
    include_stack: &mut BTreeSet<PathBuf>,
) -> Result<Vec<LocatedProject>, SessionConfigError> {
    let mut projects = Vec::new();

    for entry in entries {
        match entry {
            ProjectEntryRaw::Include(raw) => {
                let include_path = canonicalize_existing(&resolve_path(&raw.include, &base_dir)?)?;
                if !include_stack.insert(include_path.clone()) {
                    return Err(SessionConfigError::IncludeCycle(include_path));
                }

                let nested = load_include_file(&include_path, include_stack)?;
                include_stack.remove(&include_path);
                projects.extend(nested);
            }
            ProjectEntryRaw::Project(project) => projects.push(LocatedProject {
                base_dir: base_dir.clone(),
                project: project.clone(),
            }),
        }
    }

    Ok(projects)
}

fn load_include_file(
    path: &Path,
    include_stack: &mut BTreeSet<PathBuf>,
) -> Result<Vec<LocatedProject>, SessionConfigError> {
    let value = load_yaml_value(path)?;
    let base_dir = parent_dir(path);

    if matches!(
        lookup_mapping_key(&value, "projects"),
        Some(Value::Sequence(_))
    ) {
        let raw: IncludeProjectsWrapperRaw = deserialize_value(path, value)?;
        return expand_project_entries(&raw.projects, base_dir, include_stack);
    }

    match value {
        Value::Sequence(_) => {
            let raw: IncludeProjectsRaw = deserialize_value(path, value)?;
            expand_project_entries(&raw.0, base_dir, include_stack)
        }
        Value::Mapping(_) => {
            let entry: ProjectEntryRaw = deserialize_value(path, value)?;
            expand_project_entries(&[entry], base_dir, include_stack)
        }
        _ => Err(SessionConfigError::Validation(format!(
            "include file `{}` must contain a project entry, a sequence of project entries, or a `projects` list",
            path.display()
        ))),
    }
}

fn load_repo_local_config(repo: Option<&Path>) -> Result<RepoLocalRaw, SessionConfigError> {
    let Some(repo) = repo else {
        return Ok(RepoLocalRaw::default());
    };

    let path = repo.join(".nexode.yaml");
    if !path.exists() {
        return Ok(RepoLocalRaw::default());
    }

    let value = load_yaml_value(&path)?;
    deserialize_value(&path, value)
}

fn load_yaml_value(path: &Path) -> Result<Value, SessionConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| SessionConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    serde_yaml::from_str(&contents).map_err(|source| SessionConfigError::Yaml {
        path: path.to_path_buf(),
        source,
    })
}

fn deserialize_value<T>(path: &Path, value: Value) -> Result<T, SessionConfigError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_yaml::from_value(value).map_err(|source| SessionConfigError::Yaml {
        path: path.to_path_buf(),
        source,
    })
}

fn lookup_mapping_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Mapping(map) = value else {
        return None;
    };

    map.get(Value::String(key.to_string()))
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, SessionConfigError> {
    fs::canonicalize(path).map_err(|source| SessionConfigError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn parent_dir(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_path(raw: &str, base_dir: &Path) -> Result<PathBuf, SessionConfigError> {
    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|_| {
            SessionConfigError::Validation("HOME is not set; cannot expand `~` paths".to_string())
        })?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(raw)
    };

    Ok(if expanded.is_relative() {
        base_dir.join(expanded)
    } else {
        expanded
    })
}

fn validate_identifier(id: &str, kind: &str) -> Result<(), SessionConfigError> {
    let valid = !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');

    if valid {
        Ok(())
    } else {
        Err(SessionConfigError::Validation(format!(
            "{kind} id `{id}` must be kebab-case"
        )))
    }
}

#[derive(Debug)]
struct RootFile {
    version: String,
    session: SessionMetadataRaw,
    defaults: Option<DefaultsRaw>,
    models: BTreeMap<String, ModelPricingRaw>,
    projects: Vec<LocatedProject>,
}

#[derive(Debug)]
struct LocatedProject {
    base_dir: PathBuf,
    project: ProjectRaw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootV2Raw {
    version: String,
    session: SessionMetadataRaw,
    defaults: Option<DefaultsRaw>,
    models: Option<BTreeMap<String, ModelPricingRaw>>,
    projects: Vec<ProjectEntryRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootLegacyRaw {
    version: Option<String>,
    session: Option<SessionMetadataRaw>,
    defaults: Option<DefaultsRaw>,
    models: Option<BTreeMap<String, ModelPricingRaw>>,
    id: Option<String>,
    repo: Option<String>,
    display_name: Option<String>,
    color: Option<String>,
    tags: Option<Vec<String>>,
    budget: Option<BudgetRaw>,
    verify: Option<VerifyRaw>,
    slots: Option<Vec<SlotRaw>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionMetadataRaw {
    name: String,
    budget: Option<BudgetRaw>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefaultsRaw {
    model: Option<String>,
    mode: Option<YamlMode>,
    timeout_minutes: Option<u64>,
    provider_config: Option<BTreeMap<String, String>>,
    context: Option<ContextRaw>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextRaw {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetRaw {
    max_usd: Option<f64>,
    warn_usd: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyRaw {
    build: Option<String>,
    test: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelPricingRaw {
    input_per_1m: f64,
    output_per_1m: f64,
    cache_read_per_1m: Option<f64>,
}

#[derive(Debug, Clone)]
enum ProjectEntryRaw {
    Include(IncludeRaw),
    Project(ProjectRaw),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
struct IncludeProjectsRaw(Vec<ProjectEntryRaw>);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct IncludeProjectsWrapperRaw {
    projects: Vec<ProjectEntryRaw>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct IncludeRaw {
    include: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectRaw {
    id: String,
    repo: Option<String>,
    display_name: Option<String>,
    color: Option<String>,
    tags: Option<Vec<String>>,
    budget: Option<BudgetRaw>,
    defaults: Option<DefaultsRaw>,
    verify: Option<VerifyRaw>,
    slots: Option<Vec<SlotRaw>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoLocalRaw {
    display_name: Option<String>,
    color: Option<String>,
    tags: Option<Vec<String>>,
    budget: Option<BudgetRaw>,
    defaults: Option<DefaultsRaw>,
    verify: Option<VerifyRaw>,
    slots: Option<Vec<SlotRaw>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotRaw {
    id: String,
    task: String,
    model: Option<String>,
    mode: Option<YamlMode>,
    branch: Option<String>,
    timeout_minutes: Option<u64>,
    provider_config: Option<BTreeMap<String, String>>,
    context: Option<ContextRaw>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum YamlMode {
    Manual,
    Plan,
    FullAuto,
}

impl YamlMode {
    fn into_proto(self) -> AgentMode {
        match self {
            Self::Manual => AgentMode::Normal,
            Self::Plan => AgentMode::Plan,
            Self::FullAuto => AgentMode::FullAuto,
        }
    }
}

impl<'de> Deserialize<'de> for ProjectEntryRaw {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Value::Mapping(map) = &value else {
            return Err(D::Error::custom("project entry must be a mapping"));
        };

        if map.contains_key(Value::String("include".to_string())) {
            serde_yaml::from_value::<IncludeRaw>(value)
                .map(Self::Include)
                .map_err(D::Error::custom)
        } else {
            serde_yaml::from_value::<ProjectRaw>(value)
                .map(Self::Project)
                .map_err(D::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn merges_defaults_and_repo_local_slot_overrides() {
        let fixture = TestFixture::new();
        let repo = fixture.repo("app");

        fixture.write(
            repo.join(".nexode.yaml"),
            r#"
defaults:
  mode: manual
  context:
    exclude: ["dist/**"]
slots:
  - id: lint-fix
    task: Fix lint
    branch: agent/repo-lint
    context:
      exclude: ["coverage/**"]
"#,
        );

        let session_path = fixture.write(
            fixture.root().join("session.yaml"),
            format!(
                r#"
version: "2.0"
session:
  name: Demo
defaults:
  model: codex
  mode: plan
  timeout_minutes: 90
  provider_config:
    openai: "$OPENAI_API_KEY"
  context:
    exclude: ["node_modules/**"]
projects:
  - id: my-app
    repo: {}
    defaults:
      context:
        exclude: ["build/**"]
    slots:
      - id: lint-fix
        task: Fix lint
        context:
          exclude: ["tmp/**"]
"#,
                quote_yaml_path(&repo)
            ),
        );

        let config = load_session_config(&session_path).expect("session config should parse");
        let project = &config.projects[0];
        let slot = &project.slots[0];

        assert_eq!(slot.model, "codex");
        assert_eq!(slot.mode, AgentMode::Normal);
        assert_eq!(
            slot.context.exclude,
            vec![
                "node_modules/**".to_string(),
                "build/**".to_string(),
                "dist/**".to_string(),
                "tmp/**".to_string(),
                "coverage/**".to_string(),
            ]
        );
        assert_eq!(slot.branch, "agent/repo-lint");
    }

    #[test]
    fn explicit_empty_array_clears_inherited_context() {
        let fixture = TestFixture::new();
        let session_path = fixture.write(
            fixture.root().join("session.yaml"),
            r#"
version: "2.0"
session:
  name: Clear Test
defaults:
  context:
    include: ["src/**", "tests/**"]
projects:
  - id: my-app
    slots:
      - id: clear-context
        task: Clear inherited include
        context:
          include: []
"#,
        );

        let config = load_session_config(&session_path).expect("session config should parse");
        let slot = &config.projects[0].slots[0];

        assert_eq!(slot.context.include, Some(Vec::new()));
    }

    #[test]
    fn includes_resolve_relative_to_declaring_file() {
        let fixture = TestFixture::new();
        let nested = fixture.root().join("projects");
        fs::create_dir_all(&nested).expect("create include dir");
        let repo = fixture.repo("worker");

        fixture.write(
            nested.join("worker.yaml"),
            format!(
                r#"
id: worker-app
repo: {}
slots:
  - id: sync-job
    task: Run sync
"#,
                quote_yaml_path(&repo)
            ),
        );

        let session_path = fixture.write(
            fixture.root().join("session.yaml"),
            r#"
version: "2.0"
session:
  name: Include Test
projects:
  - include: ./projects/worker.yaml
"#,
        );

        let config = load_session_config(&session_path).expect("session config should parse");
        assert_eq!(config.projects[0].id, "worker-app");
        assert_eq!(config.projects[0].repo.as_deref(), Some(repo.as_path()));
    }

    #[test]
    fn wraps_legacy_session_without_projects() {
        let fixture = TestFixture::new();
        let session_path = fixture.write(
            fixture.root().join("legacy.yaml"),
            r#"
version: "1.0"
repo: .
slots:
  - id: fix-bug
    task: Fix the null pointer
"#,
        );

        let config = load_session_config(&session_path).expect("legacy config should parse");

        assert_eq!(config.session.name, "default");
        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].id, "default");
        assert_eq!(config.projects[0].slots[0].branch, "agent/fix-bug");
    }

    #[test]
    fn rejects_unknown_fields() {
        let fixture = TestFixture::new();
        let session_path = fixture.write(
            fixture.root().join("session.yaml"),
            r#"
version: "2.0"
session:
  name: Invalid
projects:
  - id: my-app
    mystery: true
    slots:
      - id: fix-bug
        task: Fix bug
"#,
        );

        let error = load_session_config(&session_path).expect_err("config should be rejected");
        assert!(error.to_string().contains("unknown field `mystery`"));
    }

    #[test]
    fn detects_include_cycles() {
        let fixture = TestFixture::new();
        fixture.write(
            fixture.root().join("a.yaml"),
            r#"
- include: ./b.yaml
"#,
        );
        fixture.write(
            fixture.root().join("b.yaml"),
            r#"
- include: ./a.yaml
"#,
        );

        let session_path = fixture.write(
            fixture.root().join("session.yaml"),
            r#"
version: "2.0"
session:
  name: Include Loop
projects:
  - include: ./a.yaml
"#,
        );

        let error = load_session_config(&session_path).expect_err("include cycle should fail");
        assert!(matches!(error, SessionConfigError::IncludeCycle(_)));
    }

    fn quote_yaml_path(path: &Path) -> String {
        format!("\"{}\"", path.display())
    }

    struct TestFixture {
        root: TempDir,
    }

    impl TestFixture {
        fn new() -> Self {
            Self {
                root: tempfile::tempdir().expect("tempdir"),
            }
        }

        fn root(&self) -> &Path {
            self.root.path()
        }

        fn repo(&self, name: &str) -> PathBuf {
            let path = self.root().join(name);
            fs::create_dir_all(&path).expect("create repo dir");
            path
        }

        fn write(&self, path: PathBuf, contents: impl AsRef<str>) -> PathBuf {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent dir");
            }

            fs::write(&path, contents.as_ref()).expect("write file");
            path
        }
    }
}
