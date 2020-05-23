#[macro_use]
extern crate lazy_static;

pub mod model;
pub mod service;

use actix_web::App;
use actix_web::{middleware, web};
use actix_web::{HttpResponse, HttpServer};

use rabbithole_endpoint_actix::ActixSettings;

use crate::service::dog::DogService;
use crate::service::human::HumanService;
use config::{Config, File};

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let mut settings = Config::default();
    settings.merge(File::with_name("config/actix.config.example.toml")).unwrap();
    let actix_settings: ActixSettings = settings.try_into().unwrap();
    let service_settings = actix_settings.clone();

    let dog_service = DogService::new();
    let human_service = HumanService::new(dog_service.clone());

    HttpServer::new(move || {
        App::new()
            .data(dog_service.clone())
            .data(human_service.clone())
            .data::<ActixSettings>(service_settings.clone())
            .wrap(middleware::Logger::default())
            .service(
                web::scope(&service_settings.path)
                    .service(DogService::actix_service())
                    .service(HumanService::actix_service()),
            )
            .default_service(web::to(HttpResponse::NotFound))
    })
    .bind(format!("[::]:{:?}", actix_settings.port))?
    .run()
    .await
}
