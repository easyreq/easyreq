use std::fmt;

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn my_trim<S>(v: &str, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(v.trim())
}

#[derive(JsonSchema, Debug, Deserialize, Serialize)]
pub struct Requirement {
    pub name: String,
    #[serde(serialize_with = "my_trim")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_info: Vec<String>,
}

#[derive(JsonSchema, Debug, Deserialize, Serialize)]
pub struct Topic {
    pub name: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub requirements: IndexMap<String, Requirement>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub subtopics: IndexMap<String, Topic>,
}

#[derive(JsonSchema, Debug, Deserialize, Serialize)]
pub struct Definition {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_info: Vec<String>,
}

#[derive(JsonSchema, Debug, Deserialize, Serialize)]
pub struct ConfigDefault {
    pub name: String,
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// Serialization as before
fn serialize_version<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&version.to_string())
}

// Custom deserialization
fn deserialize_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    struct VersionVisitor;

    impl<'de> Visitor<'de> for VersionVisitor {
        type Value = Version;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a version string in the format 'major.minor.patch'")
        }

        fn visit_str<E>(self, value: &str) -> Result<Version, E>
        where
            E: de::Error,
        {
            let parts: Vec<&str> = value.split('.').collect();
            if parts.len() != 3 {
                return Err(E::invalid_value(Unexpected::Str(value), &self));
            }

            let major = parts[0]
                .parse::<u64>()
                .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))?;
            let minor = parts[1]
                .parse::<u64>()
                .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))?;
            let patch = parts[2]
                .parse::<u64>()
                .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))?;

            Ok(Version {
                major,
                minor,
                patch,
            })
        }
    }

    deserializer.deserialize_str(VersionVisitor)
}

#[derive(JsonSchema, Debug, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    #[serde(
        serialize_with = "serialize_version",
        deserialize_with = "deserialize_version"
    )]
    #[schemars(with = "String", regex(pattern = r"^\d\.\d\.\d$"))]
    pub version: Version,
    #[serde(serialize_with = "my_trim")]
    pub description: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub topics: IndexMap<String, Topic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub definitions: Vec<Definition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_defaults: Vec<ConfigDefault>,
}

#[must_use]
pub fn demo_project() -> Project {
    serde_yaml::from_str(include_str!("../requirements.yml")).expect("Should never happen!")
}
