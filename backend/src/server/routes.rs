use actix_web::web;

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(web::scope("/auth"))
        .service(web::scope("/groups"))
        .service(web::scope("/tickets"))
        .service(web::scope("/ai"))
        .service(web::scope("/admin"));
}
