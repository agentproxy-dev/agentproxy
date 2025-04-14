use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;
use std::sync::Arc;

// JSON RPC serde inspired by https://github.com/4t145/rmcp/
#[allow(dead_code)]
pub trait ConstString: Default {
	const VALUE: &str;
	fn as_string(&self) -> &'static str {
		Self::VALUE
	}
}
#[macro_export]
macro_rules! const_string {
	($name:ident = $value:literal) => {
		#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
		pub struct $name;

		impl ConstString for $name {
			const VALUE: &str = $value;
		}

		impl serde::Serialize for $name {
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: serde::Serializer,
			{
				$value.serialize(serializer)
			}
		}

		impl<'de> serde::Deserialize<'de> for $name {
			fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
			where
				D: serde::Deserializer<'de>,
			{
				let s: String = serde::Deserialize::deserialize(deserializer)?;
				if s == $value {
					Ok($name)
				} else {
					Err(serde::de::Error::custom(format!(concat!(
						"expect const string value \"",
						$value,
						"\""
					))))
				}
			}
		}
	};
}

const_string!(JsonRpcVersion2_0 = "2.0");
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NumberOrString {
	Number(u32),
	String(Arc<str>),
}

impl NumberOrString {
	pub fn into_json_value(self) -> Value {
		match self {
			NumberOrString::Number(n) => Value::Number(serde_json::Number::from(n)),
			NumberOrString::String(s) => Value::String(s.to_string()),
		}
	}
}

impl std::fmt::Display for NumberOrString {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			NumberOrString::Number(n) => Display::fmt(&n, f),
			NumberOrString::String(s) => Display::fmt(&s, f),
		}
	}
}

impl Serialize for NumberOrString {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			NumberOrString::Number(n) => n.serialize(serializer),
			NumberOrString::String(s) => s.serialize(serializer),
		}
	}
}

impl<'de> Deserialize<'de> for NumberOrString {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value: Value = Deserialize::deserialize(deserializer)?;
		match value {
			Value::Number(n) => Ok(NumberOrString::Number(
				n.as_u64()
					.ok_or(serde::de::Error::custom("Expect an integer"))? as u32,
			)),
			Value::String(s) => Ok(NumberOrString::String(s.into())),
			_ => Err(serde::de::Error::custom("Expect number or string")),
		}
	}
}

pub type RequestId = NumberOrString;
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcRequest<R = Request> {
	pub jsonrpc: JsonRpcVersion2_0,
	pub id: RequestId,
	#[serde(flatten)]
	pub request: R,
}
#[derive(Debug, Clone)]
pub struct Request<M = String, P = JsonObject> {
	pub method: M,
	pub params: P,
}

impl<M, R> Serialize for Request<M, R>
where
	M: Serialize,
	R: Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		Proxy::serialize(
			&Proxy {
				method: &self.method,
				params: &self.params,
			},
			serializer,
		)
	}
}

#[derive(Serialize, Deserialize)]
struct Proxy<M, P> {
	method: M,
	params: P,
}

impl<'de, M, R> Deserialize<'de> for Request<M, R>
where
	M: Deserialize<'de>,
	R: Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let body = Proxy::deserialize(deserializer)?;
		Ok(Request {
			method: body.method,
			params: body.params,
		})
	}
}

pub type JsonObject<F = Value> = serde_json::Map<String, F>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JsonRpcResponse<R = JsonObject> {
	pub jsonrpc: JsonRpcVersion2_0,
	pub id: RequestId,
	pub result: R,
}
