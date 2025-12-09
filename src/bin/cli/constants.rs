pub const VERSION: &str = "v0.1.0";
pub const CONFIG_FILE: &str = ".pengy_config.json";
pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const EMBED_LOGO: &str = include_str!("../../../logo.txt");
// Default token budget used for UI display of usage percentage.
// Adjust if your model max tokens differ.
pub const MAX_TOKENS: u32 = 128_000;
