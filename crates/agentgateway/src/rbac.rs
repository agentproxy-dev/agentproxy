use crate::proto::agentgateway::dev::rbac::rule;
use crate::proto::agentgateway::dev::rbac::{Rule as XdsRule, RuleSet as XdsRuleSet};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::map::Map;
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RuleSet {
	pub name: String,
	pub namespace: String,
	pub rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct RuleSets(Vec<RuleSet>);

impl From<Vec<RuleSet>> for RuleSets {
	fn from(value: Vec<RuleSet>) -> Self {
		Self(value)
	}
}

impl RuleSets {
	pub fn validate(&self, resource: &ResourceType, claims: &Identity) -> bool {
		// If there are no rule sets, everyone has access
		if self.0.is_empty() {
			return true;
		}
		self
			.0
			.iter()
			.any(|rule_set| rule_set.validate(resource, claims))
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
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
			rule.resource.matches(resource)
				&& claims.matches(&rule.key, &rule.key_delimiter, &rule.matcher)
		})
	}
}

impl TryFrom<&XdsRuleSet> for RuleSet {
	type Error = anyhow::Error;
	fn try_from(value: &XdsRuleSet) -> Result<Self, Self::Error> {
		let rules = value
			.rules
			.iter()
			.map(|rule| -> Result<Rule, anyhow::Error> { Rule::try_from(rule) })
			.collect::<Result<Vec<Rule>, anyhow::Error>>()?;
		Ok(Self {
			name: value.name.clone(),
			namespace: value.namespace.clone(),
			rules,
		})
	}
}

impl RuleSet {
	pub fn to_key(&self) -> String {
		format!("{}.{}", self.namespace, self.name)
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Rule {
	key: String,
	key_delimiter: String,
	matcher: Matcher,
	resource: ResourceType,
}

impl TryFrom<&XdsRule> for Rule {
	type Error = anyhow::Error;
	fn try_from(value: &XdsRule) -> Result<Self, Self::Error> {
		let matcher = Matcher::new(&value.matcher.try_into()?, &value.value);
		let resource = value.resource.as_ref().unwrap().try_into()?;
		Ok(Rule {
			key: value.key.clone(),
			key_delimiter: value.key_delimiter.clone(),
			matcher,
			resource,
		})
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum ResourceType {
	Tool(ResourceId),
	Prompt(ResourceId),
	Resource(ResourceId),
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ResourceId {
	#[serde(default)]
	target: String,
	#[serde(default)]
	id: String,
}

impl ResourceId {
	pub fn new(target: String, id: String) -> Self {
		Self { target, id }
	}
}

impl TryFrom<&rule::Resource> for ResourceType {
	type Error = anyhow::Error;
	fn try_from(value: &rule::Resource) -> Result<Self, Self::Error> {
		match value.r#type.to_lowercase().as_str() {
			"tool" => Ok(ResourceType::Tool(ResourceId::new(
				value.target.clone(),
				value.id.clone(),
			))),
			"prompt" => Ok(ResourceType::Prompt(ResourceId::new(
				value.target.clone(),
				value.id.clone(),
			))),
			"resource" => Ok(ResourceType::Resource(ResourceId::new(
				value.target.clone(),
				value.id.clone(),
			))),
			_ => Err(anyhow::anyhow!("Invalid resource type")),
		}
	}
}

impl ResourceType {
	fn matches(&self, other: &Self) -> bool {
		// Support wildcard
		match (self, other) {
			(ResourceType::Tool(a), ResourceType::Tool(b)) => a.matches(b),
			(ResourceType::Prompt(a), ResourceType::Prompt(b)) => a.matches(b),
			(ResourceType::Resource(a), ResourceType::Resource(b)) => a.matches(b),
			_ => false,
		}
	}
}

impl ResourceId {
	// This method must always be called from the rule context, never from the
	fn matches(&self, other: &Self) -> bool {
		// matching logic is as follows:
		// If the id does not match or contain a wildcard, then the resource is not a match
		// Empty string is a wildcard
		if !match (self.id.as_str(), other.id.as_str()) {
			("*", _) => true,
			("", _) => true,
			(id1, id2) if id1 == id2 => true,
			_ => false,
		} {
			return false;
		}

		// If the target does not match or contain a wildcard, then the resource is not a match
		// Empty string is a wildcard
		if !match (self.target.as_str(), other.target.as_str()) {
			("*", _) => true,
			("", _) => true,
			(target1, target2) if target1 == target2 => true,
			_ => false,
		} {
			return false;
		}

		true
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Matcher {
	Equals(String),
	Contains(String),
}

impl Matcher {
	pub fn new(value: &rule::Matcher, value_str: &str) -> Self {
		match value {
			rule::Matcher::Equals => Matcher::Equals(value_str.to_string()),
			rule::Matcher::Contains => Matcher::Contains(value_str.to_string()),
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct Claims {
	pub inner: Map<String, Value>,
	pub jwt: SecretString,
}

impl Claims {
	pub fn new(claims: Map<String, Value>, jwt: SecretString) -> Self {
		Self { inner: claims, jwt }
	}
}

#[derive(Clone, Debug, Default)]
pub struct Identity {
	pub claims: Option<Claims>,
	pub connection_id: Option<String>,
}

impl Identity {
	pub fn empty() -> Self {
		Self {
			claims: None,
			connection_id: None,
		}
	}

	pub fn new(claims: Option<Claims>, connection_id: Option<String>) -> Self {
		Self {
			claims,
			connection_id,
		}
	}

	// Matches the claim against the rule
	// If the claim is not found, it returns false
	// If the claim is found, it returns true if the claim matches the rule
	// If the claim is found, it returns false if the claim does not match the rule
	pub fn matches(&self, key: &str, key_delimiter: &str, matcher: &Matcher) -> bool {
		match matcher {
			Matcher::Equals(value) => self.get_claim(key, key_delimiter) == Some(value),
			Matcher::Contains(value) => {
				// First try to get as string
				if let Some(claim_str) = self.get_claim(key, key_delimiter) {
					return claim_str.contains(value);
				}

				// If not a string, try to get as raw value and check if it's an array
				if let Some(Value::Array(arr)) = self.get_claim_value(key, key_delimiter) {
					// Check if any array element contains the value
					return arr.iter().any(|item| {
						if let Some(item_str) = item.as_str() {
							item_str == value || item_str.contains(value)
						} else {
							false
						}
					});
				}

				false
			},
		}
	}

	// Attempts to get the claim from the claims map
	// The key should be split by the key_delimiter and then the map should be searched recursively
	// If the key is not found, it returns None
	// If the key is found, it returns the value
	pub fn get_claim(&self, key: &str, key_delimiter: &str) -> Option<&str> {
		match &self.claims {
			Some(claims) => {
				// Split the key by the delimiter to handle nested lookups
				let keys = key.split(key_delimiter).collect::<Vec<&str>>();

				// Start with the root claims map
				let mut current_value = &claims.inner;

				// Navigate through each key level
				let num_keys = keys.len();
				for (index, key_part) in keys.into_iter().enumerate() {
					// Get the value at this level
					let value = current_value.get(key_part)?;

					// If this is the last key part, return the string value
					if index == num_keys - 1 {
						return value.as_str();
					}

					// Otherwise, try to navigate deeper if it's an object
					current_value = value.as_object()?;
				}

				None
			},
			None => None,
		}
	}

	// Get the raw JSON value for a claim (used for array handling)
	pub fn get_claim_value(&self, key: &str, key_delimiter: &str) -> Option<&Value> {
		match &self.claims {
			Some(claims) => {
				// Split the key by the delimiter to handle nested lookups
				let keys = key.split(key_delimiter).collect::<Vec<&str>>();

				// Start with the root claims map
				let mut current_value = &claims.inner;

				// Navigate through each key level
				let num_keys = keys.len();
				for (index, key_part) in keys.into_iter().enumerate() {
					// Get the value at this level
					let value = current_value.get(key_part)?;

					// If this is the last key part, return the raw value
					if index == num_keys - 1 {
						return Some(value);
					}

					// Otherwise, try to navigate deeper if it's an object
					current_value = value.as_object()?;
				}

				None
			},
			None => None,
		}
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_rbac_reject_exact_match() {
		let rules = vec![Rule {
			key: "user".to_string(),
			key_delimiter: ".".to_string(),
			matcher: Matcher::Equals("admin".to_string()),
			resource: ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string(),
			)),
		}];
		let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
		let mut headers = Map::new();
		headers.insert("sub".to_string(), "1234567890".to_string().into());
		let id = Identity::new(
			Some(Claims::new(headers, SecretString::new("".into()))),
			None,
		);
		assert!(!rbac.validate(
			&ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string()
			)),
			&id
		));
	}

	#[test]
	fn test_rbac_check_exact_match() {
		let rules = vec![Rule {
			key: "sub".to_string(),
			key_delimiter: ".".to_string(),
			matcher: Matcher::Equals("1234567890".to_string()),
			resource: ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string(),
			)),
		}];
		let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
		let mut headers = Map::new();
		headers.insert("sub".to_string(), "1234567890".to_string().into());
		let id = Identity::new(
			Some(Claims::new(headers, SecretString::new("".into()))),
			None,
		);
		assert!(rbac.validate(
			&ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string()
			)),
			&id
		));
	}

	#[test]
	fn test_rbac_check_wildcard_match() {
		let cases: Vec<(ResourceId, ResourceId, bool)> = vec![
			(
				ResourceId::new("server".to_string(), "increment".to_string()),
				ResourceId::new("server".to_string(), "increment".to_string()),
				true,
			),
			(
				ResourceId::new("server".to_string(), "*".to_string()),
				ResourceId::new("server".to_string(), "increment".to_string()),
				true,
			),
			(
				ResourceId::new("server".to_string(), "increment".to_string()),
				ResourceId::new("server".to_string(), "decrement".to_string()),
				false,
			),
			(
				ResourceId::new("".to_string(), "increment".to_string()),
				ResourceId::new("server".to_string(), "increment".to_string()),
				true,
			),
			(
				ResourceId::new("other_server".to_string(), "increment".to_string()),
				ResourceId::new("server".to_string(), "increment".to_string()),
				false,
			),
		];
		for (rule, other_rule, expected) in cases {
			assert_eq!(
				rule.matches(&other_rule),
				expected,
				"rule: {:?}, other_rule: {:?}",
				rule,
				other_rule
			);
		}
	}

	#[test]
	fn test_rbac_check_contains_match() {
		let rules = vec![Rule {
			key: "groups".to_string(),
			key_delimiter: ".".to_string(),
			matcher: Matcher::Contains("admin".to_string()),
			resource: ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string(),
			)),
		}];
		let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
		let mut headers = Map::new();
		headers.insert(
			"groups".to_string(),
			"user,admin,developer".to_string().into(),
		);
		let id = Identity::new(
			Some(Claims::new(headers, SecretString::new("".into()))),
			None,
		);
		assert!(rbac.validate(
			&ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string()
			)),
			&id
		));
	}

	#[test]
	fn test_rbac_check_nested_key_match() {
		let rules = vec![Rule {
			key: "user.role".to_string(),
			key_delimiter: ".".to_string(),
			matcher: Matcher::Equals("admin".to_string()),
			resource: ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string(),
			)),
		}];
		let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
		let mut headers = Map::new();
		let mut user_obj = Map::new();
		user_obj.insert("role".to_string(), "admin".into());
		headers.insert("user".to_string(), user_obj.into());
		let id = Identity::new(
			Some(Claims::new(headers, SecretString::new("".into()))),
			None,
		);
		assert!(rbac.validate(
			&ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string()
			)),
			&id
		));
	}

	#[test]
	fn test_rbac_check_array_contains_match() {
		let rules = vec![Rule {
			key: "roles".to_string(),
			key_delimiter: ".".to_string(),
			matcher: Matcher::Contains("admin".to_string()),
			resource: ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string(),
			)),
		}];
		let rbac = RuleSet::new("test".to_string(), "test".to_string(), rules);
		let mut headers = Map::new();
		// Create an array of roles
		let roles: Vec<Value> = vec!["user".into(), "admin".into(), "developer".into()];
		headers.insert("roles".to_string(), roles.into());
		let id = Identity::new(
			Some(Claims::new(headers, SecretString::new("".into()))),
			None,
		);
		assert!(rbac.validate(
			&ResourceType::Tool(ResourceId::new(
				"server".to_string(),
				"increment".to_string()
			)),
			&id
		));
	}
}
