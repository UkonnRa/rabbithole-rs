use crate::model::link::Links;
use crate::model::relationship::Relationships;
use crate::model::{error, Meta};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::FromIterator;
use std::str::FromStr;

pub type ResourceIdentifiers = Vec<ResourceIdentifier>;
pub type Resources = Vec<Resource>;

lazy_static! {
    static ref INVALID_ATTR_FIELDS: HashSet<&'static str> =
        HashSet::from_iter(vec!["relationships", "links", "type", "id"]);
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AttributeField(pub serde_json::Value);

impl AttributeField {
    pub fn cmp_with_str(&self, value: &str, field: &str) -> Result<Ordering> {
        let value: AttributeField = value.parse()?;
        self.partial_cmp(&value).ok_or_else(|| {
            error::Error::FieldNotMatch(field, &self.to_string(), &value.to_string(), None)
        })
    }

    pub fn eq_with_str(&self, value: &str, field: &str) -> Result<bool> {
        if value.contains('*') && self.0.is_string() {
            let value = value.replace('*', "\\w*");
            let regex: regex::Regex = value.parse::<regex::Regex>().unwrap();
            Ok(regex.is_match(&self.0.as_str().unwrap()))
        } else {
            self.cmp_with_str(value, field).map(|o| o == Ordering::Equal)
        }
    }
}

impl FromStr for AttributeField {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(s).map_err(|err| error::Error::InvalidJson(&err, None))?;
        Ok(value.into())
    }
}

impl From<serde_json::Value> for AttributeField {
    fn from(value: serde_json::Value) -> Self { Self(value) }
}

impl ToString for AttributeField {
    fn to_string(&self) -> String { serde_json::to_string(&self.0).unwrap() }
}

impl PartialOrd for AttributeField {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match &self.0 {
            serde_json::Value::String(a) => {
                let b: String = if let serde_json::Value::String(b) = &other.0 {
                    b.clone()
                } else {
                    other.0.to_string()
                };
                a.partial_cmp(&b)
            },
            serde_json::Value::Number(a) if f64::from_str(&other.0.to_string()).is_ok() => {
                a.as_f64().unwrap().partial_cmp(&f64::from_str(&other.0.to_string()).unwrap())
            },
            serde_json::Value::Bool(a) if bool::from_str(&other.0.to_string()).is_ok() => {
                a.partial_cmp(&bool::from_str(&other.0.to_string()).unwrap())
            },
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Attributes(HashMap<String, AttributeField>);

impl From<HashMap<String, AttributeField>> for Attributes {
    fn from(map: HashMap<String, AttributeField>) -> Self { Self(map) }
}

impl<K: ToString> From<HashMap<K, serde_json::Value>> for Attributes {
    fn from(map: HashMap<K, serde_json::Value>) -> Self {
        map.into_iter()
            .map(|(k, v)| (k.to_string(), v.into()))
            .collect::<HashMap<String, AttributeField>>()
            .into()
    }
}

impl Attributes {
    pub fn get_field(&self, field_name: &str) -> Result<&AttributeField> {
        self.0.get(field_name).ok_or_else(|| error::Error::FieldNotExist(field_name, None))
    }

    pub fn cmp(&self, field: &str, other: &Self) -> Result<Ordering> {
        let self_field = self.get_field(field)?;
        let other_field = other.get_field(field)?;
        if let Some(ord) = self_field.partial_cmp(&other_field) {
            Ok(ord)
        } else {
            Err(error::Error::FieldNotMatch(
                field,
                &self_field.to_string(),
                &other_field.to_string(),
                None,
            ))
        }
    }

    pub fn get_json_value_map(&self) -> Result<HashMap<String, serde_json::Value>> {
        self.0
            .iter()
            .map(|(k, v)| match serde_json::to_value(v) {
                Ok(vv) => Ok((k.clone(), vv)),
                Err(err) => Err(error::Error::InvalidJson(&err, None)),
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    pub fn retain(mut self, keys: &HashSet<String>) -> Self {
        self.0.retain(|k, _| keys.contains(k));
        self
    }
}

/// Valid Resource Identifier (can be None)
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum IdentifierData {
    Single(Option<ResourceIdentifier>),
    Multiple(ResourceIdentifiers),
}

impl IdentifierData {
    pub fn data(&self) -> Vec<ResourceIdentifier> {
        match self {
            IdentifierData::Single(Some(data)) => vec![data.clone()],
            IdentifierData::Single(None) => Default::default(),
            IdentifierData::Multiple(data) => data.clone(),
        }
    }
}

impl Default for IdentifierData {
    fn default() -> Self { IdentifierData::Single(None) }
}

/// Resource Identifier
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct ResourceIdentifier {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub id: String,
}

impl ResourceIdentifier {
    pub fn new(ty: &str, id: &str) -> Self { Self { ty: ty.into(), id: id.into() } }
}

/// JSON-API Resource
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    #[serde(flatten)]
    pub id: ResourceIdentifier,
    #[serde(skip_serializing_if = "Attributes::is_empty")]
    #[serde(default)]
    pub attributes: Attributes,
    #[serde(skip_serializing_if = "Relationships::is_empty")]
    #[serde(default)]
    pub relationships: Relationships,
    #[serde(skip_serializing_if = "Links::is_empty")]
    #[serde(default)]
    pub links: Links,
    #[serde(skip_serializing_if = "Meta::is_empty")]
    #[serde(default)]
    pub meta: Meta,
}

impl Resource {
    pub fn retain_attributes(mut self, attributes: &HashSet<String>) -> Self {
        self.attributes = self.attributes.retain(attributes);
        self
    }

    pub fn extend_meta(&mut self, meta: Meta) { self.meta.extend(meta.into_iter()); }

    pub fn extend_links(&mut self, links: Links) { self.links.extend(links.into_iter()); }
}

#[cfg(test)]
mod tests {
    use crate::model::resource::{Resource, ResourceIdentifier};
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn serde_test() {
        let attributes: HashMap<String, serde_json::Value> =
            HashMap::from_iter(vec![("name".into(), serde_json::Value::String("name1".into()))]);
        let res = Resource {
            id: ResourceIdentifier::new("ty", "id"),
            attributes: attributes.into(),
            relationships: Default::default(),
            links: Default::default(),
            meta: Default::default(),
        };

        let res_json = serde_json::to_value(&res).unwrap();
        assert_eq!(res_json["id"], "id");
    }
}
