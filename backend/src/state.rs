use mongodb::Database;

use crate::config::Config;

pub struct AppState {
    pub db: Database,
    pub config: Config,
}
