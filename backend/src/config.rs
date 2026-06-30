pub struct Config {
    pub mongo_uri: String,
}

impl Config {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        dotenvy::dotenv().ok();
        Ok(Config {
            mongo_uri: std::env::var("MONGO_URI")?,
        })
    }

    pub fn bind_address(&self) -> String {
        "127.0.0.1:8080".to_string()
    }
}
