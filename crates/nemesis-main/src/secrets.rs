use aws_sdk_secretsmanager::Client;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct ExchangeSecrets {
    pub api_key: String,
    pub api_secret: String,
}

pub struct SecretsManager {
    client: Client,
}

impl SecretsManager {
    pub async fn new() -> anyhow::Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Ok(Self {
            client: Client::new(&config),
        })
    }

    pub async fn get_exchange_secrets(&self, secret_name: &str) -> anyhow::Result<ExchangeSecrets> {
        info!(secret = %secret_name, "Fetching secrets from AWS Secrets Manager");

        let resp = self
            .client
            .get_secret_value()
            .secret_id(secret_name)
            .send()
            .await?;

        let secret_string = resp
            .secret_string()
            .ok_or_else(|| anyhow::anyhow!("Secret has no string value"))?;

        let secrets: ExchangeSecrets = serde_json::from_str(secret_string)?;
        Ok(secrets)
    }
}
