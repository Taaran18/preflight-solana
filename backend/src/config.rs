use crate::solana::Pubkey;

pub struct Config {
    pub host: [u8; 4],
    pub allowed_origins: Vec<String>,
    pub port: u16,
    pub rpc_url: String,
    pub jup_base: String,
    pub jup_key: Option<String>,
    pub jup_rps: f64,
    pub anthropic_key: Option<String>,
    pub model: String,
    pub effort: String,
    pub guard_program: Option<Pubkey>,
}

fn var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

impl Config {
    pub fn from_env() -> Self {
        let jup_key = var("JUPITER_API_KEY");
        let default_rps = if jup_key.is_some() { 1.0 } else { 0.5 };
        let host = var("HOST")
            .and_then(|v| v.parse::<std::net::Ipv4Addr>().ok())
            .map(|ip| ip.octets())
            .unwrap_or([127, 0, 0, 1]);
        let allowed_origins = var("ALLOWED_ORIGINS")
            .unwrap_or_else(|| "http://localhost:3000,http://127.0.0.1:3000".to_string())
            .split(',')
            .map(|origin| origin.trim().trim_end_matches('/').to_string())
            .filter(|origin| !origin.is_empty())
            .collect();
        Self {
            host,
            allowed_origins,
            port: var("PORT").and_then(|v| v.parse().ok()).unwrap_or(8080),
            rpc_url: var("SOLANA_RPC_URL").unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string()),
            jup_base: var("JUPITER_API_BASE").unwrap_or_else(|| "https://api.jup.ag".to_string()),
            jup_rps: var("JUPITER_RPS").and_then(|v| v.parse().ok()).unwrap_or(default_rps),
            jup_key,
            anthropic_key: var("ANTHROPIC_API_KEY"),
            model: var("ANTHROPIC_MODEL").unwrap_or_else(|| "claude-sonnet-5-5".to_string()),
            effort: var("ANTHROPIC_EFFORT").unwrap_or_else(|| "low".to_string()),
            guard_program: var("GUARD_PROGRAM_ID").and_then(|v| Pubkey::parse(&v)),
        }
    }

    pub fn rpc_host(&self) -> String {
        self.rpc_url
            .split("://")
            .nth(1)
            .unwrap_or(&self.rpc_url)
            .split(['/', '?'])
            .next()
            .unwrap_or_default()
            .to_string()
    }

    pub fn print_summary(&self) {
        let flag = |on: bool| if on { "on" } else { "off" };
        println!("RPC          {}", self.rpc_host());
        println!("Claude       {} ({})", flag(self.anthropic_key.is_some()), self.model);
        println!("Jupiter key  {}", flag(self.jup_key.is_some()));
        match &self.guard_program {
            Some(program) => println!("Guard        {program}"),
            None => println!("Guard        off (memo receipts)"),
        }
    }
}
