use actix_web::web;

use crate::auth::handlers as auth_handlers;

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/auth")
                .route("/register", web::post().to(auth_handlers::register))
                .route("/login", web::post().to(auth_handlers::login)),
        )
        .service(web::scope("/groups"))
        .service(web::scope("/tickets"))
        .service(web::scope("/ai"))
        .service(web::scope("/admin"));
}
