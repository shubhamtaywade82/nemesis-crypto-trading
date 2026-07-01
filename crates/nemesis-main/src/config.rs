use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub exchange: ExchangeConfig,
    pub symbols: Vec<SymbolConfig>,
    pub risk: RiskConfig,
    pub logging: LogConfig,
}

#[derive(Debug, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub api_key: String,
    pub api_secret: String,
    pub testnet: bool,
}

#[derive(Debug, Deserialize)]
pub struct SymbolConfig {
    pub symbol: String,
    pub ws_url: String,
    pub bar_type: String,
    pub bar_param: f64,
}

#[derive(Debug, Deserialize)]
pub struct RiskConfig {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_spread_bps: f64,
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
}

impl AppConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
