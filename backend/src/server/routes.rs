use actix_web::web;

use crate::activity::handlers as activity_handlers;
use crate::admin::handlers as admin_handlers;
use crate::ai::handlers as ai_handlers;
use crate::auth::handlers as auth_handlers;
use crate::comment::handlers as comment_handlers;
use crate::group::handlers as group_handlers;
use crate::link::handlers as link_handlers;
use crate::reference::handlers as reference_handlers;
use crate::ticket::handlers as ticket_handlers;

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/auth")
                .route("/register", web::post().to(auth_handlers::register))
                .route("/login", web::post().to(auth_handlers::login))
                .route("/me", web::get().to(auth_handlers::me))
                .route("/me", web::patch().to(auth_handlers::update_me))
                .route(
                    "/me/password",
                    web::post().to(auth_handlers::change_password),
                )
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
                )
                .route(
                    "/{id}/tickets",
                    web::post().to(ticket_handlers::create_ticket),
                )
                .route("/{id}/tickets", web::get().to(ticket_handlers::list_tickets))
                .route(
                    "/{id}/tickets/{ticket_id}",
                    web::get().to(ticket_handlers::get_ticket),
                )
                .route(
                    "/{id}/tickets/{ticket_id}",
                    web::patch().to(ticket_handlers::update_ticket),
                )
                .route(
                    "/{id}/tickets/{ticket_id}",
                    web::delete().to(ticket_handlers::delete_ticket),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/comments",
                    web::post().to(comment_handlers::create_comment),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/comments",
                    web::get().to(comment_handlers::list_comments),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/comments/{comment_id}",
                    web::delete().to(comment_handlers::delete_comment),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/activity",
                    web::get().to(activity_handlers::list_activity),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/links",
                    web::post().to(link_handlers::create_link),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/links",
                    web::get().to(link_handlers::list_links),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/links/{link_id}",
                    web::delete().to(link_handlers::delete_link),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/references",
                    web::post().to(reference_handlers::create_reference),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/references",
                    web::get().to(reference_handlers::list_references),
                )
                .route(
                    "/{id}/tickets/{ticket_id}/references/{reference_id}",
                    web::delete().to(reference_handlers::delete_reference),
                ),
        )
        .service(
            web::scope("/ai")
                .service(
                    web::scope("/groups/{id}/tickets/{ticket_id}")
                        .route("/summarize", web::post().to(ai_handlers::summarize_ticket))
                        .route("/analyze", web::post().to(ai_handlers::analyze_ticket))
                        .route(
                            "/conversations",
                            web::post().to(ai_handlers::create_conversation),
                        )
                        .route(
                            "/conversations",
                            web::get().to(ai_handlers::list_conversations),
                        )
                        .route(
                            "/conversations/{conversation_id}/messages",
                            web::post().to(ai_handlers::send_conversation_message),
                        )
                        .route(
                            "/conversations/{conversation_id}/messages",
                            web::get().to(ai_handlers::list_conversation_messages),
                        )
                        .route(
                            "/conversations/{conversation_id}",
                            web::delete().to(ai_handlers::delete_conversation),
                        ),
                ),
        )
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
