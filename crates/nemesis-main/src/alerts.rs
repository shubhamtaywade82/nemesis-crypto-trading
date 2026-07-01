use reqwest::Client;
use serde_json::json;
use tracing::{error, warn};

#[allow(dead_code)]
pub struct AlertDispatcher {
    client: Client,
    webhook_url: Option<String>,
}

#[allow(dead_code)]
impl AlertDispatcher {
    pub fn new(webhook_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            webhook_url,
        }
    }

    pub async fn send_critical(&self, title: &str, message: &str, severity: &str) {
        let Some(url) = &self.webhook_url else {
            warn!(
                title,
                message, severity, "No webhook configured, alert logged only"
            );
            return;
        };

        let payload = json!({
            "text": format!("🚨 [{}] {}\n{}", severity.to_uppercase(), title, message),
            "blocks": [{
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("*🚨 {} - {}*\n{}", severity.to_uppercase(), title, message)
                }
            }]
        });

        match self.client.post(url).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(title, "Alert sent successfully");
            }
            Ok(resp) => {
                error!(title, status = %resp.status(), "Alert webhook returned error");
            }
            Err(e) => {
                error!(title, error = %e, "Failed to send alert");
            }
        }
    }

    pub async fn kill_switch_triggered(&self, reason: &str) {
        self.send_critical(
            "KILL SWITCH ACTIVATED",
            &format!(
                "Reason: {}\nAll trading halted. Manual intervention required.",
                reason
            ),
            "critical",
        )
        .await;
    }

    pub async fn stale_feed_detected(&self, symbol: &str, duration_secs: u64) {
        self.send_critical(
            "STALE FEED DETECTED",
            &format!(
                "Symbol: {}\nDuration: {}s\nBar builder paused.",
                symbol, duration_secs
            ),
            "warning",
        )
        .await;
    }

    pub async fn reconciliation_drift(&self, drift_type: &str, details: &str) {
        self.send_critical(
            "RECONCILIATION DRIFT",
            &format!("Type: {}\nDetails: {}", drift_type, details),
            "high",
        )
        .await;
    }
}
