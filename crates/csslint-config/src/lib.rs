#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use csslint_core::{RuleId, Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub rules: BTreeMap<RuleId, Severity>,
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
