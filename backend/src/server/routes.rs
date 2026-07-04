use actix_web::web;

use crate::auth::handlers as auth_handlers;

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/auth")
                .route("/register", web::post().to(auth_handlers::register))
                .route("/login", web::post().to(auth_handlers::login))
                .route("/me", web::get().to(auth_handlers::me))
                .route("/refresh", web::post().to(auth_handlers::refresh))
                .route("/logout", web::post().to(auth_handlers::logout)),
        )
        .service(web::scope("/groups"))
        .service(web::scope("/tickets"))
        .service(web::scope("/ai"))
        .service(web::scope("/admin"));
}
