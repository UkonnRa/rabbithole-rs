use rabbithole::model::resource::Resource;
use rabbithole::operation::ResourceDataWrapper;
use rabbithole::query::page::Cursor;
use rabbithole_endpoint_actix::ActixSettings;
use std::str::FromStr;

fn check_names(resources: &[Resource], names: &[usize]) {
    assert_eq!(resources.len(), names.len());

    for r in resources {
        let name =
            usize::from_str(r.attributes.get_field("name").unwrap().0.as_str().unwrap()).unwrap();
        assert!(names.contains(&name));
    }
}

use actix_web::test;
use rabbithole::model::document::Document;

pub mod common;
use common::model;
use common::model::dog::generate_dogs;
use common::service;
use common::{get, post};

#[macro_use]
extern crate lazy_static;

#[actix_rt::test]
async fn empty_test() {
    // Prepare data
    let mut app = init_app!(CursorBased);

    let after_cursor = Cursor { id: "1".to_string() }.to_string();
    let before_cursor = Cursor { id: "2".to_string() }.to_string();

    let req = get(format!(
        "/api/v1/dogs?sort=name&page[after]={}&page[before]={}&page[size]=3",
        after_cursor, before_cursor
    )
    .as_str());

    let doc: Document = test::read_response_json(&mut app, req).await;
    assert_eq!(doc.links.len(), 1);

    let (resources, _) = doc.into_multiple().unwrap();
    assert!(resources.is_empty());
}

#[actix_rt::test]
async fn range_test() {
    // Prepare data
    let mut app = init_app!(CursorBased);

    let dogs = generate_dogs(7);
    let dog_resources = ResourceDataWrapper::from_entities(&dogs, "http://localhost:1234/api/v1");
    for dog in &dog_resources {
        let req = post("/api/v1/dogs", &dog);
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    let after_cursor = Cursor { id: dogs[1].id.to_string() }.to_string();
    let before_cursor = Cursor { id: dogs[4].id.to_string() }.to_string();

    let req = get(format!(
        "/api/v1/dogs?sort=name&page[after]={}&page[before]={}&page[size]=3",
        after_cursor, before_cursor
    )
    .as_str());
    let doc: Document = test::read_response_json(&mut app, req).await;
    assert_eq!(doc.links.len(), 3);

    if let Some(prev) = doc.links.get("prev") {
        let prev = http::Uri::from(prev).path_and_query().unwrap().to_string();
        let req = get(prev.as_str());
        let doc: Document = test::read_response_json(&mut app, req).await;
        let (resources, _) = doc.into_multiple().unwrap();
        check_names(&resources, &[0, 1]);
    } else {
        unreachable!("`prev` link is needed");
    }

    if let Some(next) = doc.links.get("next") {
        let next = http::Uri::from(next).path_and_query().unwrap().to_string();
        let req = get(next.as_str());
        let doc: Document = test::read_response_json(&mut app, req).await;
        let (resources, _) = doc.into_multiple().unwrap();
        check_names(&resources, &[4, 5, 6]);
    } else {
        unreachable!("`next` link is needed");
    }

    let (resources, _) = doc.into_multiple().unwrap();
    check_names(&resources, &[2, 3]);
}

#[actix_rt::test]
async fn one_side_test() {
    // Prepare data
    let mut app = init_app!(CursorBased);

    let dogs = generate_dogs(7);
    let dog_resources = ResourceDataWrapper::from_entities(&dogs, "http://localhost:1234/api/v1");
    for dog in &dog_resources {
        let req = post("/api/v1/dogs", &dog);
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    // Only after
    let after_cursor = Cursor { id: dogs[3].id.to_string() }.to_string();
    let req =
        get(format!("/api/v1/dogs?sort=name&page[after]={}&page[size]=3", after_cursor).as_str());
    let doc: Document = test::read_response_json(&mut app, req).await;
    assert_eq!(doc.links.len(), 2);

    if let Some(prev) = doc.links.get("prev") {
        let prev = http::Uri::from(prev).path_and_query().unwrap().to_string();
        let req = get(prev.as_str());
        let doc: Document = test::read_response_json(&mut app, req).await;
        let (resources, _) = doc.into_multiple().unwrap();
        check_names(&resources, &[1, 2, 3]);
    } else {
        unreachable!("`prev` link is needed");
    }

    let (resources, _) = doc.into_multiple().unwrap();
    check_names(&resources, &[4, 5, 6]);

    // Only before
    let before_cursor = Cursor { id: dogs[2].id.to_string() }.to_string();
    let req =
        get(format!("/api/v1/dogs?sort=name&page[before]={}&page[size]=3", before_cursor).as_str());
    let doc: Document = test::read_response_json(&mut app, req).await;
    assert_eq!(doc.links.len(), 2);

    if let Some(next) = doc.links.get("next") {
        let next = http::Uri::from(next).path_and_query().unwrap().to_string();
        let req = get(next.as_str());
        let doc: Document = test::read_response_json(&mut app, req).await;
        let (resources, _) = doc.into_multiple().unwrap();
        check_names(&resources, &[2, 3, 4]);
    } else {
        unreachable!("`next` link is needed");
    }

    let (resources, _) = doc.into_multiple().unwrap();
    check_names(&resources, &[0, 1]);

    // Bad cursor will be ignored
    let before_cursor = Cursor { id: "no exist".to_string() }.to_string();
    let req =
        get(format!("/api/v1/dogs?sort=name&page[before]={}&page[size]=3", before_cursor).as_str());
    let doc: Document = test::read_response_json(&mut app, req).await;
    assert_eq!(doc.links.len(), 2);

    if let Some(next) = doc.links.get("next") {
        let next = http::Uri::from(next).path_and_query().unwrap().to_string();
        let req = get(next.as_str());
        let doc: Document = test::read_response_json(&mut app, req).await;
        let (resources, _) = doc.into_multiple().unwrap();
        check_names(&resources, &[3, 4, 5]);
    } else {
        unreachable!("`next` link is needed");
    }

    let (resources, _) = doc.into_multiple().unwrap();
    check_names(&resources, &[0, 1, 2]);
}
