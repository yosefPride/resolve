use actix_web::web;

use crate::admin::handlers as admin_handlers;
use crate::auth::handlers as auth_handlers;
use crate::group::handlers as group_handlers;

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
        .service(
            web::scope("/groups")
                .route("", web::post().to(group_handlers::create_group))
                .route("", web::get().to(group_handlers::list_my_groups))
                .route("/{id}", web::get().to(group_handlers::get_group))
                .route("/{id}", web::patch().to(group_handlers::rename_group))
                .route("/{id}", web::delete().to(group_handlers::delete_group))
                .route("/{id}/users", web::get().to(group_handlers::list_members))
                .route("/{id}/users", web::post().to(group_handlers::add_member))
                .route(
                    "/{id}/users/lookup",
                    web::get().to(group_handlers::lookup_user),
                )
                .route(
                    "/{id}/users/{user_id}",
                    web::patch().to(group_handlers::update_member_role),
                )
                .route(
                    "/{id}/users/{user_id}",
                    web::delete().to(group_handlers::remove_member),
                ),
        )
        .service(web::scope("/tickets"))
        .service(web::scope("/ai"))
        .service(
            web::scope("/admin")
                .route("/users", web::get().to(admin_handlers::list_users))
                .route("/groups", web::get().to(admin_handlers::list_groups))
                .route("/groups/{id}", web::delete().to(admin_handlers::delete_group))
                .route("/audit-log", web::get().to(admin_handlers::list_audit_log))
                .service(
                    web::scope("/users/{id}")
                        .route("/deletion-check", web::get().to(admin_handlers::deletion_check))
                        .route("/delete", web::post().to(admin_handlers::delete_user)),
                ),
        );
}
