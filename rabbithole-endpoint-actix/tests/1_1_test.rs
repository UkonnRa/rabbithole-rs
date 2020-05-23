use actix_web::http::{header, StatusCode};
use actix_web::test::call_service;
use actix_web::test::TestRequest;
use rabbithole::JSON_API_HEADER;
use rabbithole_endpoint_actix::ActixSettings;

pub mod common;
use common::{model, service};

#[macro_use]
extern crate lazy_static;

#[actix_rt::test]
/// https://jsonapi.org/format/#content-negotiation-servers
async fn invalid_accept_header_test() {
    let mut app = init_app!(1, 1);
    let req = TestRequest::get()
        .uri("/api/v1/people")
        .header(header::CONTENT_TYPE, JSON_API_HEADER)
        .to_request();

    let resp = call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
}

#[actix_rt::test]
/// https://jsonapi.org/format/#content-negotiation-servers
async fn invalid_content_type_test() {
    let mut app = init_app!(1, 1);
    let req = TestRequest::get()
        .uri("/api/v1/people")
        .header(header::ACCEPT, JSON_API_HEADER)
        .to_request();

    let resp = call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[actix_rt::test]
/// https://jsonapi.org/format/#content-negotiation-servers
async fn invalid_content_type_params_test() {
    let mut app = init_app!(1, 1);
    let req = TestRequest::get()
        .uri("/api/v1/people")
        .header(header::ACCEPT, format!(r#"{}; profile="cursor-pagination""#, JSON_API_HEADER))
        .header(
            header::CONTENT_TYPE,
            format!(r#"{}; profile="cursor-pagination""#, JSON_API_HEADER),
        )
        .to_request();

    let resp = call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
