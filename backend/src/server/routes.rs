use actix_web::web;

use crate::admin::handlers as admin_handlers;
use crate::auth::handlers as auth_handlers;
use crate::group::handlers as group_handlers;

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(web::scope("/auth"))
        .service(web::scope("/groups"))
        .service(web::scope("/tickets"))
        .service(web::scope("/ai"))
        .service(
            web::scope("/admin")
                .route("/users", web::get().to(admin_handlers::list_users))
                .route("/groups", web::get().to(admin_handlers::list_groups))
                .route("/groups/{id}", web::delete().to(admin_handlers::delete_group))
                .service(
                    web::scope("/users/{id}")
                        .route("/deletion-check", web::get().to(admin_handlers::deletion_check))
                        .route("/delete", web::post().to(admin_handlers::delete_user)),
                ),
        );
}
