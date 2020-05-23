#[macro_use]
extern crate lazy_static;

use rabbithole::model::document::Document;
use rabbithole::model::resource::{AttributeField, Resource};
use rabbithole::operation::ResourceDataWrapper;
use rabbithole_endpoint_actix::ActixSettings;

pub mod common;
use common::model;
use common::model::dog::generate_dogs;
use common::service;
use common::{get, post};

use actix_web::test::{call_service, read_response_json};

fn get_names(resources: &[Resource]) -> Vec<String> {
    let names: Result<Vec<AttributeField>, rabbithole::model::error::Error> =
        resources.iter().map(|r| r.attributes.get_field("name").map(Clone::clone)).collect();
    names.unwrap().iter().map(|a| a.0.as_str().unwrap().to_string()).collect()
}

#[actix_rt::test]
async fn default_test() {
    // Prepare data
    let mut app = init_app!(DefaultPage);

    let dogs = generate_dogs(7);

    for dog in ResourceDataWrapper::from_entities(&dogs, "https://localhost:1234/api/v1") {
        let req = post("/api/v1/dogs", &dog);
        let resp = call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    let req = get("/api/v1/dogs");
    let doc: Document = read_response_json(&mut app, req).await;

    assert_eq!(doc.links.len(), 1);
    assert!(doc.links.contains_key("self"));

    let (resources, _) = doc.into_multiple().unwrap();
    assert_eq!(resources.len(), 7);
    let names = get_names(&resources);
    for i in 0 .. 7 {
        assert!(names.contains(&i.to_string()));
    }
}
