pub mod handlers;
pub mod metrics;
pub mod relay;

use a2a_sdk::{A2aRequest, AgentCard};
use reqwest::{Response, Url};

#[derive(Debug)]
pub struct Client {
	pub url: Url,
	pub client: reqwest::Client,
}

impl Client {
	pub async fn send_request(&self, req: &A2aRequest) -> Result<Response, reqwest::Error> {
		self.client.post(self.url.clone()).json(req).send().await
	}
	async fn fetch_agent_card(&self) -> Result<AgentCard, anyhow::Error> {
		Ok(
			self
				.client
				.get(format!("{}.well-known/agent.json", self.url))
				.header("Content-type", "application/json")
				.send()
				.await?
				.json()
				.await?,
		)
	}
}
