use crate::xds::mcp::kgateway_dev::rbac::rule;
use crate::xds::mcp::kgateway_dev::rbac::{Config as XdsRuleSet, Rule as XdsRule};
use itertools::{self, Itertools};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::map::Map;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleSet {
	pub name: String,
	pub namespace: String,
	pub rules: Vec<Rule>,
}

impl RuleSet {
	pub fn new(name: String, namespace: String, rules: Vec<Rule>) -> Self {
		Self {
			name,
			namespace,
			rules,
		}
	}

	// Check if the claims have access to the resource
	pub fn validate(&self, resource: &ResourceType, claims: &Identity) -> bool {
		tracing::info!("Checking RBAC for resource: {:?}", resource);
		// If there are no rules, everyone has access
		if self.rules.is_empty() {
			return true;
		}

		self.rules.iter().any(|rule| {
			rule.resource.matches(resource) && claims.matches(&rule.key, &rule.value, &rule.matcher)
		})
	}
}

impl From<&XdsRuleSet> for RuleSet {
	fn from(value: &XdsRuleSet) -> Self {
		Self {
			name: value.name.clone(),
			namespace: value.namespace.clone(),
			rules: value.rules.iter().map_into().collect(),
		}
	}
}

impl RuleSet {
	pub fn to_key(&self) -> String {
		format!("{}.{}", self.namespace, self.name)
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
	key: String,
	value: String,
	matcher: Matcher,
	resource: ResourceType,
}

impl From<&XdsRule> for Rule {
	fn from(value: &XdsRule) -> Self {
		Rule {
			key: value.key.clone(),
			value: value.value.clone(),
			matcher: Matcher::from(&value.matcher.try_into().unwrap()),
			resource: value.resource.as_ref().unwrap().try_into().unwrap(),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
	Tool { id: String },
	Prompt { id: String },
	Resource { id: String },
}

impl TryFrom<&rule::Resource> for ResourceType {
	type Error = anyhow::Error;
	fn try_from(value: &rule::Resource) -> Result<Self, Self::Error> {
		match value.r#type.try_into() {
			Ok(rule::resource::ResourceType::Tool) => Ok(ResourceType::Tool {
				id: value.id.clone(),
			}),
			Ok(rule::resource::ResourceType::Prompt) => Ok(ResourceType::Prompt {
				id: value.id.clone(),
			}),
			Ok(rule::resource::ResourceType::Resource) => Ok(ResourceType::Resource {
				id: value.id.clone(),
			}),
			_ => Err(anyhow::anyhow!("Invalid resource type")),
		}
	}
}

impl ResourceType {
	pub fn matches(&self, other: &Self) -> bool {
		// Support wildcard
		match (self, other) {
			(ResourceType::Tool { id: a }, ResourceType::Tool { id: b }) => a == b || a == "*",
			(ResourceType::Prompt { id: a }, ResourceType::Prompt { id: b }) => a == b || a == "*",
			(ResourceType::Resource { id: a }, ResourceType::Resource { id: b }) => a == b || a == "*",
			_ => false,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Matcher {
	Equals,
}

impl From<&rule::Matcher> for Matcher {
	fn from(value: &rule::Matcher) -> Self {
		match value {
			rule::Matcher::Equals => Matcher::Equals,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
	claims: Option<Map<String, Value>>,
	connection_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims(pub Map<String, Value>);

impl Claims {
	pub fn new(claims: Map<String, Value>) -> Self {
		Self(claims)
	}
}

impl Identity {
	pub fn empty() -> Self {
		Self {
			claims: None,
			connection_id: None,
		}
	}

	pub fn new(claims: Option<Map<String, Value>>, connection_id: Option<String>) -> Self {
		Self {
			claims,
			connection_id,
		}
	}

	pub fn matches(&self, key: &str, value: &str, matcher: &Matcher) -> bool {
		match matcher {
			Matcher::Equals => self.get_claim(key) == Some(value),
		}
	}
	fn get_claim(&self, key: &str) -> Option<&str> {
		match &self.claims {
			Some(claims) => claims.get(key).and_then(|v| v.as_str()),
			None => None,
		}
	}
}

#[test]
fn test_rbac_false_check() {
	let rules = vec![Rule {
		key: "user".to_string(),
		value: "admin".to_string(),
		matcher: Matcher::Equals,
		resource: ResourceType::Tool {
			id: "increment".to_string(),
		},
	}];
	let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
	let mut headers = Map::new();
	headers.insert("sub".to_string(), "1234567890".to_string().into());
	let id = Identity::new(Some(headers), None);
	assert!(!rbac.validate(
		&ResourceType::Tool {
			id: "increment".to_string()
		},
		&id
	));
}

#[test]
fn test_rbac_check() {
	let rules = vec![Rule {
		key: "sub".to_string(),
		value: "1234567890".to_string(),
		matcher: Matcher::Equals,
		resource: ResourceType::Tool {
			id: "increment".to_string(),
		},
	}];
	let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
	let mut headers = Map::new();
	headers.insert("sub".to_string(), "1234567890".to_string().into());
	let id = Identity::new(Some(headers), None);
	assert!(rbac.validate(
		&ResourceType::Tool {
			id: "increment".to_string()
		},
		&id
	));
}
