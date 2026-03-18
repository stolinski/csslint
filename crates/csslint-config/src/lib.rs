#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use csslint_core::{RuleId, Severity};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub rules: BTreeMap<RuleId, Severity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetName {
    Recommended,
    Strict,
    Minimal,
}

impl PresetName {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "recommended" => Ok(Self::Recommended),
            "strict" => Ok(Self::Strict),
            "minimal" => Ok(Self::Minimal),
            _ => Err(format!(
                "Invalid preset '{raw}'. Expected recommended, strict, or minimal."
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameworkName {
    Vue,
    Svelte,
}

impl FrameworkName {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "vue" => Ok(Self::Vue),
            "svelte" => Ok(Self::Svelte),
            _ => Err(format!(
                "Invalid framework '{raw}'. Expected one of: vue, svelte."
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    Defaults,
    Explicit(PathBuf),
    Discovered(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub config: Config,
    pub source: ConfigSource,
    pub preset: Option<PresetName>,
    pub frameworks: Vec<FrameworkName>,
    pub targets: Option<String>,
    pub fix: Option<bool>,
}

impl LoadedConfig {
    pub fn defaults() -> Self {
        Self {
            config: Config::default(),
            source: ConfigSource::Defaults,
            preset: None,
            frameworks: Vec::new(),
            targets: None,
            fix: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub key: Option<String>,
    pub message: String,
    pub file_path: Option<PathBuf>,
}

impl ConfigDiagnostic {
    fn new(file_path: Option<PathBuf>, key: Option<String>, message: impl Into<String>) -> Self {
        Self {
            key,
            message: message.into(),
            file_path,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut rules = BTreeMap::new();
        rules.insert(RuleId::from("no_empty_rules"), Severity::Warn);
        rules.insert(RuleId::from("no_global_leaks"), Severity::Error);

        Self { rules }
    }
}

pub fn config_file_name() -> &'static str {
    ".csslint"
}

pub fn parse_severity(raw: &str) -> Result<Severity, String> {
    match raw {
        "off" => Ok(Severity::Off),
        "warn" => Ok(Severity::Warn),
        "error" => Ok(Severity::Error),
        _ => Err(format!(
            "Invalid severity '{raw}'. Expected off, warn, or error."
        )),
    }
}

pub fn load_for_target(
    target_path: &Path,
    explicit_config_path: Option<&Path>,
) -> Result<LoadedConfig, Vec<ConfigDiagnostic>> {
    let resolved_source = resolve_config_source(target_path, explicit_config_path)?;
    let Some(source) = resolved_source else {
        return Ok(LoadedConfig::defaults());
    };

    let config_path = match &source {
        ConfigSource::Explicit(path) | ConfigSource::Discovered(path) => path.clone(),
        ConfigSource::Defaults => {
            return Ok(LoadedConfig::defaults());
        }
    };

    let raw = fs::read_to_string(&config_path).map_err(|error| {
        vec![ConfigDiagnostic::new(
            Some(config_path.clone()),
            None,
            format!("Failed to read config file: {error}"),
        )]
    })?;

    parse_config_document(&raw, source)
}

pub fn discover_nearest_config_path(target_path: &Path) -> Option<PathBuf> {
    let mut current = if target_path.is_file() {
        target_path.parent()?.to_path_buf()
    } else if target_path.is_dir() {
        target_path.to_path_buf()
    } else if target_path.extension().is_some() {
        target_path.parent()?.to_path_buf()
    } else {
        target_path.to_path_buf()
    };

    loop {
        let candidate = current.join(config_file_name());
        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub fn format_diagnostics(diagnostics: &[ConfigDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let path_part = diagnostic
                .file_path
                .as_ref()
                .map(|path| format!("{}: ", path.display()))
                .unwrap_or_default();
            let key_part = diagnostic
                .key
                .as_ref()
                .map(|key| format!("[{key}] "))
                .unwrap_or_default();
            format!("{path_part}{key_part}{}", diagnostic.message)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn resolve_config_source(
    target_path: &Path,
    explicit_config_path: Option<&Path>,
) -> Result<Option<ConfigSource>, Vec<ConfigDiagnostic>> {
    if let Some(explicit_path) = explicit_config_path {
        return validate_explicit_path(explicit_path).map(Some);
    }

    Ok(discover_nearest_config_path(target_path).map(ConfigSource::Discovered))
}

fn validate_explicit_path(path: &Path) -> Result<ConfigSource, Vec<ConfigDiagnostic>> {
    if !path.exists() {
        return Err(vec![ConfigDiagnostic::new(
            Some(path.to_path_buf()),
            None,
            "Explicit --config path does not exist.",
        )]);
    }

    if !path.is_file() {
        return Err(vec![ConfigDiagnostic::new(
            Some(path.to_path_buf()),
            None,
            "Explicit --config path must point to a file.",
        )]);
    }

    Ok(ConfigSource::Explicit(path.to_path_buf()))
}

fn parse_config_document(
    raw: &str,
    source: ConfigSource,
) -> Result<LoadedConfig, Vec<ConfigDiagnostic>> {
    let file_path = match &source {
        ConfigSource::Explicit(path) | ConfigSource::Discovered(path) => Some(path.clone()),
        ConfigSource::Defaults => None,
    };

    let parsed: Value = serde_json::from_str(raw).map_err(|error| {
        vec![ConfigDiagnostic::new(
            file_path.clone(),
            None,
            format!("Invalid JSON syntax in config file: {error}"),
        )]
    })?;

    let Some(root) = parsed.as_object() else {
        return Err(vec![ConfigDiagnostic::new(
            file_path,
            None,
            "Config root must be a JSON object.",
        )]);
    };

    let mut diagnostics = Vec::new();
    let mut rule_overrides: BTreeMap<RuleId, Severity> = BTreeMap::new();
    let mut preset = None;
    let mut frameworks = Vec::new();
    let mut targets = None;
    let mut fix = None;

    let allowed_keys = ["preset", "frameworks", "targets", "fix", "rules"];
    for key in root.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some(key.clone()),
                format!(
                    "Unknown top-level key '{key}'. Allowed keys: preset, frameworks, targets, fix, rules."
                ),
            ));
        }
    }

    if let Some(value) = root.get("preset") {
        match value {
            Value::String(raw_preset) => match PresetName::parse(raw_preset) {
                Ok(valid_preset) => preset = Some(valid_preset),
                Err(message) => diagnostics.push(ConfigDiagnostic::new(
                    file_path.clone(),
                    Some("preset".to_string()),
                    message,
                )),
            },
            _ => diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some("preset".to_string()),
                "preset must be a string.",
            )),
        }
    }

    if let Some(value) = root.get("frameworks") {
        match value {
            Value::Array(items) => {
                let mut dedupe = BTreeSet::new();
                for (index, item) in items.iter().enumerate() {
                    let key = format!("frameworks[{index}]");
                    match item {
                        Value::String(raw_framework) => match FrameworkName::parse(raw_framework) {
                            Ok(framework) => {
                                if dedupe.insert(framework) {
                                    frameworks.push(framework);
                                }
                            }
                            Err(message) => diagnostics.push(ConfigDiagnostic::new(
                                file_path.clone(),
                                Some(key),
                                message,
                            )),
                        },
                        _ => diagnostics.push(ConfigDiagnostic::new(
                            file_path.clone(),
                            Some(key),
                            "framework entries must be strings.",
                        )),
                    }
                }
            }
            _ => diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some("frameworks".to_string()),
                "frameworks must be an array.",
            )),
        }
    }

    if let Some(value) = root.get("targets") {
        match value {
            Value::String(raw_targets) => {
                targets = Some(raw_targets.clone());
            }
            _ => diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some("targets".to_string()),
                "targets must be a string.",
            )),
        }
    }

    if let Some(value) = root.get("fix") {
        match value {
            Value::Bool(raw_fix) => {
                fix = Some(*raw_fix);
            }
            _ => diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some("fix".to_string()),
                "fix must be a boolean.",
            )),
        }
    }

    if let Some(value) = root.get("rules") {
        match value {
            Value::Object(raw_rules) => {
                parse_rules_object(
                    raw_rules,
                    file_path.clone(),
                    &mut diagnostics,
                    &mut rule_overrides,
                );
            }
            _ => diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some("rules".to_string()),
                "rules must be an object map of ruleId -> severity.",
            )),
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut config = Config::default();
    for (rule_id, severity) in rule_overrides {
        config.rules.insert(rule_id, severity);
    }

    Ok(LoadedConfig {
        config,
        source,
        preset,
        frameworks,
        targets,
        fix,
    })
}

fn parse_rules_object(
    raw_rules: &serde_json::Map<String, Value>,
    file_path: Option<PathBuf>,
    diagnostics: &mut Vec<ConfigDiagnostic>,
    target: &mut BTreeMap<RuleId, Severity>,
) {
    for (rule_id, value) in raw_rules {
        let key = format!("rules.{rule_id}");

        if !is_known_core_rule_id(rule_id) {
            diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some(key),
                format!("Unknown rule id '{rule_id}'"),
            ));
            continue;
        }

        match value {
            Value::String(raw_severity) => match parse_severity(raw_severity) {
                Ok(severity) => {
                    target.insert(RuleId::from(rule_id.clone()), severity);
                }
                Err(message) => {
                    diagnostics.push(ConfigDiagnostic::new(file_path.clone(), Some(key), message))
                }
            },
            _ => diagnostics.push(ConfigDiagnostic::new(
                file_path.clone(),
                Some(key),
                "Rule severity must be a string ('off', 'warn', or 'error').",
            )),
        }
    }
}

fn is_known_core_rule_id(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "no_unknown_properties"
            | "no_invalid_values"
            | "no_duplicate_selectors"
            | "no_duplicate_declarations"
            | "no_empty_rules"
            | "no_legacy_vendor_prefixes"
            | "no_overqualified_selectors"
            | "prefer_logical_properties"
            | "no_global_leaks"
            | "no_deprecated_features"
    )
}

pub fn from_raw_rules(raw: &BTreeMap<String, String>) -> Result<Config, Vec<ConfigDiagnostic>> {
    let mut rules = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for (key, value) in raw {
        if !is_known_core_rule_id(key) {
            diagnostics.push(ConfigDiagnostic::new(
                None,
                Some(key.clone()),
                format!("Unknown rule id '{key}'"),
            ));
            continue;
        }

        match parse_severity(value) {
            Ok(severity) => {
                rules.insert(RuleId::from(key.clone()), severity);
            }
            Err(message) => {
                diagnostics.push(ConfigDiagnostic::new(None, Some(key.clone()), message))
            }
        }
    }

    if diagnostics.is_empty() {
        Ok(Config { rules })
    } else {
        Err(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use csslint_core::Severity;

    use crate::{
        config_file_name, discover_nearest_config_path, format_diagnostics, from_raw_rules,
        load_for_target, parse_severity, ConfigSource, FrameworkName, PresetName,
    };

    #[test]
    fn parses_supported_severity_values() {
        assert_eq!(parse_severity("off"), Ok(Severity::Off));
        assert_eq!(parse_severity("warn"), Ok(Severity::Warn));
        assert_eq!(parse_severity("error"), Ok(Severity::Error));
    }

    #[test]
    fn rejects_invalid_severity_values() {
        let error = parse_severity("fatal").expect_err("fatal should be invalid");
        assert!(error.contains("Invalid severity"));
    }

    #[test]
    fn surfaces_diagnostics_for_invalid_raw_rule_values() {
        let mut raw = BTreeMap::new();
        raw.insert("no_empty_rules".to_string(), "warn".to_string());
        raw.insert("no_global_leaks".to_string(), "fatal".to_string());

        let diagnostics = from_raw_rules(&raw).expect_err("invalid severity should fail");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].key.as_deref(), Some("no_global_leaks"));
    }

    #[test]
    fn rejects_unknown_rule_ids_including_deferred_plugin_candidates() {
        let mut raw = BTreeMap::new();
        raw.insert("no_empty_rules".to_string(), "warn".to_string());
        raw.insert("no_unused_scoped_selectors".to_string(), "warn".to_string());

        let diagnostics = from_raw_rules(&raw).expect_err("unknown rule id should fail");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].key.as_deref(),
            Some("no_unused_scoped_selectors")
        );
        assert!(diagnostics[0].message.contains("Unknown rule id"));
    }

    #[test]
    fn discovers_nearest_config_file_by_directory_traversal() {
        let fixture = TempFixture::new("config-discovery");
        fixture.write(config_file_name(), "{}");
        fixture.write("src/components/Button.css", ".btn { color: red; }");

        let nearest =
            discover_nearest_config_path(&fixture.path().join("src/components/Button.css"));
        let nearest = nearest.expect("config should be discovered");
        assert_eq!(
            fixture.relative_path(&nearest),
            config_file_name().to_string()
        );
    }

    #[test]
    fn load_uses_defaults_when_no_config_exists() {
        let fixture = TempFixture::new("config-defaults");
        fixture.write("src/app.css", ".app { color: red; }");

        let loaded =
            load_for_target(&fixture.path().join("src"), None).expect("defaults should load");
        assert_eq!(loaded.source, ConfigSource::Defaults);
        assert!(loaded.preset.is_none());
        assert!(loaded.frameworks.is_empty());
        assert!(loaded.targets.is_none());
    }

    #[test]
    fn load_prefers_explicit_config_path() {
        let fixture = TempFixture::new("config-explicit");
        fixture.write(".csslint", "{ \"preset\": \"recommended\" }");
        fixture.write(
            "custom/config.json",
            "{ \"preset\": \"strict\", \"frameworks\": [\"vue\"], \"targets\": \"defaults\", \"fix\": true, \"rules\": { \"no_empty_rules\": \"error\" } }",
        );

        let explicit = fixture.path().join("custom/config.json");
        let loaded =
            load_for_target(fixture.path(), Some(&explicit)).expect("explicit config should win");
        assert_eq!(loaded.source, ConfigSource::Explicit(explicit));
        assert_eq!(loaded.preset, Some(PresetName::Strict));
        assert_eq!(loaded.frameworks, vec![FrameworkName::Vue]);
        assert_eq!(loaded.targets.as_deref(), Some("defaults"));
        assert_eq!(loaded.fix, Some(true));
        assert_eq!(
            loaded.config.rules.get(&"no_empty_rules".into()),
            Some(&Severity::Error)
        );
    }

    #[test]
    fn load_reports_invalid_json_and_unknown_keys() {
        let fixture = TempFixture::new("config-errors");
        let config_path = fixture.path().join(config_file_name());
        fixture.write(config_file_name(), "{ \"unknown\": true ");

        let errors = load_for_target(fixture.path(), None).expect_err("invalid json should fail");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file_path.as_ref(), Some(&config_path));
        assert!(errors[0].message.contains("Invalid JSON syntax"));

        fixture.write(config_file_name(), "{ \"unknown\": true }");
        let errors = load_for_target(fixture.path(), None).expect_err("unknown key should fail");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].key.as_deref(), Some("unknown"));
    }

    #[test]
    fn load_reports_rule_and_severity_errors() {
        let fixture = TempFixture::new("config-rule-errors");
        fixture.write(
            config_file_name(),
            "{ \"rules\": { \"no_empty_rules\": \"fatal\", \"no_unused_scoped_selectors\": \"warn\" } }",
        );

        let diagnostics =
            load_for_target(fixture.path(), None).expect_err("invalid rules should fail");
        assert_eq!(diagnostics.len(), 2);
        assert!(format_diagnostics(&diagnostics).contains("Unknown rule id"));
        assert!(format_diagnostics(&diagnostics).contains("Invalid severity"));
    }

    #[test]
    fn load_reports_type_mismatches() {
        let fixture = TempFixture::new("config-type-errors");
        fixture.write(
            config_file_name(),
            "{ \"preset\": 1, \"frameworks\": \"vue\", \"targets\": false, \"fix\": \"yes\", \"rules\": [] }",
        );

        let diagnostics =
            load_for_target(fixture.path(), None).expect_err("type errors should fail");
        assert_eq!(diagnostics.len(), 5);
    }

    struct TempFixture {
        root: PathBuf,
    }

    impl TempFixture {
        fn new(label: &str) -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "csslint-config-{label}-{pid}-{suffix}",
                pid = std::process::id()
            ));
            fs::create_dir_all(&root).expect("temp fixture directory should be created");
            Self { root }
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write(&self, relative: &str, contents: &str) {
            let full = self.root.join(relative);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).expect("parent directory should be created");
            }
            fs::write(full, contents).expect("fixture file should be written");
        }

        fn relative_path(&self, absolute: &Path) -> String {
            absolute
                .strip_prefix(&self.root)
                .expect("absolute path should be inside fixture root")
                .to_string_lossy()
                .replace('\\', "/")
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
