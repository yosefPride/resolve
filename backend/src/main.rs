mod ai;
mod admin;
mod auth;
mod comment;
mod config;
mod db;
mod errors;
mod group;
mod rbac;
mod server;
mod state;
mod ticket;
mod user;
mod utils;

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};

use crate::config::Config;
use crate::state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config::from_env().map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Missing required environment variable: {error}"),
        )
    })?;

    let client = db::connect(&config)
        .await
        .map_err(|error| std::io::Error::other(format!("MongoDB connection failed: {error}")))?;
    let database = db::database(&client, &config);
    let bind_address = config.bind_address();
    let app_state = web::Data::new(AppState {
        db: database,
        config,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .service(
                web::scope("/api/v1")
                    .configure(server::routes::configure),
            )
    })
    .bind(bind_address)?
    .run()
    .await
}
