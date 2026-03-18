#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use csslint_core::{RuleId, Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub rules: BTreeMap<RuleId, Severity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub key: Option<String>,
    pub message: String,
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
            diagnostics.push(ConfigDiagnostic {
                key: Some(key.clone()),
                message: format!("Unknown rule id '{key}'"),
            });
            continue;
        }

        match parse_severity(value) {
            Ok(severity) => {
                rules.insert(RuleId::from(key.clone()), severity);
            }
            Err(message) => diagnostics.push(ConfigDiagnostic {
                key: Some(key.clone()),
                message,
            }),
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

    use csslint_core::Severity;

    use crate::{from_raw_rules, parse_severity};

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
}
