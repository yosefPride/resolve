mod admin;
mod ai;
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
use actix_web::http::header;
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
    db::ensure_indexes(&database)
        .await
        .map_err(|error| std::io::Error::other(format!("Failed to create indexes: {error}")))?;
    let bind_address = config.bind_address();
    let app_state = web::Data::new(AppState {
        db: database,
        config,
    });

    HttpServer::new(move || {
        // A wildcard origin (Cors::permissive()'s default) cannot be combined
        // with credentialed requests per the CORS spec — and the refresh
        // cookie requires `credentials: 'include'` on the frontend's fetch
        // calls to be sent/received at all. So the origin must be explicit.
        let cors = Cors::default()
            .allowed_origin(&app_state.config.frontend_origin)
            .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .wrap(cors)
            .service(web::scope("/api/v1").configure(server::routes::configure))
    })
    .bind(bind_address)?
    .run()
    .await
}
