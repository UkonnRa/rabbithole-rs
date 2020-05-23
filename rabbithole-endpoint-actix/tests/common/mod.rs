pub mod model;
pub mod service;

#[macro_export]
macro_rules! init_app {
    (PageBased) => {{
        init_app!("tests/config/actix.config.test.page_based.toml".to_string())
    }};
    (CursorBased) => {{
        init_app!("tests/config/actix.config.test.cursor_based.toml".to_string())
    }};
    (OffsetBased) => {{
        init_app!("tests/config/actix.config.test.offset_based.toml".to_string())
    }};
    (DefaultPage) => {{
        init_app!("tests/config/actix.config.test.default_page.toml".to_string())
    }};
    ($major:expr, $minor:expr) => {{
        init_app!(format!("tests/config/actix.config.test.v{}_{}.toml", $major, $minor))
    }};
    ($file_name:expr) => {{
        let mut settings = config::Config::default();
        settings.merge(config::File::with_name(&$file_name)).unwrap();

        let actix_settings: ActixSettings = settings.try_into().unwrap();
        let dog_service = service::dog::DogService::new();
        let human_service = service::human::HumanService::new(dog_service.clone());

        actix_web::test::init_service(
            actix_web::App::new()
                .data(dog_service.clone())
                .data(human_service.clone())
                .data(actix_settings.clone())
                .service(
                    actix_web::web::scope(&actix_settings.path)
                        .service(service::dog::DogService::actix_service())
                        .service(service::human::HumanService::actix_service()),
                )
                .default_service(actix_web::web::to(actix_web::HttpResponse::NotFound)),
        )
        .await
    }};
}

use actix_web::test::TestRequest;
use serde::Serialize;

pub fn request(req: TestRequest, uri: &str) -> TestRequest {
    req.uri(uri)
        .header("content-type", "application/vnd.api+json")
        .header("accept", "application/vnd.api+json")
}

pub fn post<D: Serialize>(uri: &str, data: &D) -> actix_http::Request {
    println!("POST {}", uri);
    request(TestRequest::post(), uri).set_payload(serde_json::to_string(data).unwrap()).to_request()
}

pub fn patch<D: Serialize>(uri: &str, data: &D) -> actix_http::Request {
    println!("PATCH {}", uri);
    request(TestRequest::patch(), uri)
        .set_payload(serde_json::to_string(data).unwrap())
        .to_request()
}

pub fn get(uri: &str) -> actix_http::Request {
    println!("GET {}", uri);
    request(TestRequest::get(), uri).to_request()
}

pub fn delete<D: Serialize>(uri: &str, data: &D) -> actix_http::Request {
    println!("DELETE {}", uri);
    request(TestRequest::delete(), uri)
        .set_payload(serde_json::to_string(data).unwrap())
        .to_request()
}
