use crate::security::constant_time_eq;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Permission {
    Read,
    Write,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TopicTokens {
    read: Option<String>,
    write: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DefaultPolicy {
    Allow,
    #[default]
    Deny,
}

#[derive(Debug, Default, Deserialize)]
struct AccessFile {
    #[serde(default)]
    default: DefaultPolicy,
    #[serde(default)]
    topics: HashMap<String, TopicTokens>,
}

#[derive(Clone, Debug)]
pub struct AccessControl {
    global: Option<String>,
    default: DefaultPolicy,
    topics: HashMap<String, TopicTokens>,
    configured: bool,
}

impl AccessControl {
    pub fn load(global: Option<String>, path: Option<&str>) -> Result<Self, String> {
        let (file, configured) = match path {
            Some(path) => {
                let raw = std::fs::read_to_string(path)
                    .map_err(|error| format!("failed to read {path}: {error}"))?;
                let file = serde_json::from_str(&raw)
                    .map_err(|error| format!("invalid topic access file {path}: {error}"))?;
                (file, true)
            }
            None => (AccessFile::default(), false),
        };
        Ok(Self {
            global,
            default: file.default,
            topics: file.topics,
            configured,
        })
    }

    pub fn allows_global(&self, token: Option<&str>) -> bool {
        match &self.global {
            Some(expected) => matches(token, expected),
            None => !self.configured,
        }
    }

    pub fn allows_topic(&self, topic: &str, permission: Permission, token: Option<&str>) -> bool {
        if self
            .global
            .as_ref()
            .is_some_and(|expected| matches(token, expected))
        {
            return true;
        }
        match self.topics.get(topic) {
            Some(tokens) => match permission {
                Permission::Read => tokens
                    .read
                    .as_ref()
                    .is_some_and(|expected| matches(token, expected)),
                Permission::Write => tokens
                    .write
                    .as_ref()
                    .is_some_and(|expected| matches(token, expected)),
            },
            None => {
                (!self.configured && self.global.is_none())
                    || (self.configured && self.default == DefaultPolicy::Allow)
            }
        }
    }
}

fn matches(provided: Option<&str>, expected: &str) -> bool {
    provided.is_some_and(|provided| constant_time_eq(provided.as_bytes(), expected.as_bytes()))
}

#[cfg(test)]
#[path = "../tests/unit/access.rs"]
mod tests;
