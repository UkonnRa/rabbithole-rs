#[macro_use]
extern crate lazy_static;

use actix_web::http::StatusCode;
use rabbithole::model::document::DocumentItem;
use rabbithole::model::resource::{AttributeField, Attributes, IdentifierData, ResourceIdentifier};
use rabbithole::operation::{IdentifierDataWrapper, ResourceDataWrapper};
use rabbithole_endpoint_actix::ActixSettings;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use actix_web::test::{call_service, read_response_json};
use rabbithole::model::document::Document;

pub mod common;
use common::model;
use common::model::dog::{generate_dogs, Dog};
use common::model::human::Human;
use common::service;
use common::{delete, get, patch, post};

#[actix_rt::test]
async fn basic_crud_test() {
    let mut app = init_app!(1, 0);

    // When field type not found
    let req = get("/api/v1/aliens/1");
    let resp = actix_web::test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Create a list of entities
    let dogs = generate_dogs(5);
    let dog_resources = ResourceDataWrapper::from_entities(&dogs, "http://localhost:1234/api/v1");
    for dog in dog_resources.clone() {
        let req = post("/api/v1/dogs", &dog);
        let resp = call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    // Verify that 5 dogs created
    let req = get("/api/v1/dogs");
    let resp: Document = read_response_json(&mut app, req).await;
    assert_eq!(resp.into_multiple().unwrap().0.len(), 5);

    // Verify that data is null on possible relationship
    let req = get("/api/v1/people/1");
    let resp: Document = read_response_json(&mut app, req).await;
    assert_eq!(resp.item, DocumentItem::PrimaryData(None));

    // Update the name of the first dog
    let mut first_dog_res = dog_resources.first().cloned().unwrap();
    let new_name = json!("Fido");

    let map: HashMap<String, serde_json::Value> =
        HashMap::from_iter(vec![("name".into(), new_name.clone())]);
    let map: Attributes = map.into();
    first_dog_res.data.attributes = map;

    let req = patch(format!("/api/v1/dogs/{}", first_dog_res.data.id.id).as_str(), &first_dog_res);
    let resp = call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = get(format!("/api/v1/dogs/{}", first_dog_res.data.id.id).as_str());
    let resp: Document = read_response_json(&mut app, req).await;
    let (resource, _) = resp.into_single().unwrap();
    let AttributeField(name) = resource.attributes.get_field("name").unwrap();
    assert_eq!(name, &new_name);

    // Get all dogs
    let req = get("/api/v1/dogs");
    let doc: Document = read_response_json(&mut app, req).await;
    let (new_dog_resources, _) = doc.into_multiple().unwrap();
    assert_eq!(new_dog_resources.len(), dog_resources.len());

    let new_dog_resource =
        new_dog_resources.iter().find(|r| r.id.id != first_dog_res.data.id.id).unwrap();
    let old_matched_resource =
        &dog_resources.iter().find(|r| r.data.id == new_dog_resource.id).unwrap().data;
    assert_eq!(new_dog_resource, old_matched_resource);

    // Get all dogs
    let req = get("/api/v1/dogs");
    let doc: Document = read_response_json(&mut app, req).await;
    let (new_dog_resources, _) = doc.into_multiple().unwrap();
    assert_eq!(new_dog_resources.len(), dog_resources.len());

    let new_dog_resource =
        new_dog_resources.iter().find(|r| r.id.id != first_dog_res.data.id.id).unwrap();
    let old_matched_resource =
        &dog_resources.iter().find(|r| r.data.id == new_dog_resource.id).unwrap().data;
    assert_eq!(new_dog_resource, old_matched_resource);

    // Add some humans
    let humans: Vec<Human> = dogs.windows(2).map(Human::from).collect();
    let human_resources =
        ResourceDataWrapper::from_entities(&humans, "http://localhost:1234/api/v1");
    for human in &human_resources {
        let req = post("/api/v1/people", &human);
        let doc: Document = read_response_json(&mut app, req).await;
        let (resource, _) = doc.into_single().unwrap();
        assert_eq!(resource.id, human.data.id);
        assert!(resource.relationships.contains_key("dogs"));

        let dogs_relat = resource.relationships.get("dogs").unwrap();
        let dogs_relat = dogs_relat.data.data();
        assert_eq!(dogs_relat.len(), 2);
    }

    // Get the first human
    let first_human_id = humans.first().unwrap().id.to_string();
    let req = get(format!("/api/v1/people/{}", first_human_id).as_str());
    let doc: Document = read_response_json(&mut app, req).await;
    let (first_human_resource, included) = doc.into_single().unwrap();
    assert_eq!(
        &first_human_resource.as_ref().attributes,
        &human_resources.first().unwrap().data.attributes
    );
    assert_eq!(
        &first_human_resource.as_ref().relationships,
        &human_resources.first().unwrap().data.relationships
    );
    for (pet_key, pet_value) in &included {
        let old_dog = new_dog_resources.iter().find(|wrapper| &wrapper.id == pet_key);
        let old_dog = old_dog.unwrap();
        assert_eq!(&pet_value.attributes, &old_dog.attributes);
    }
}

#[actix_rt::test]
async fn relationship_test() {
    // Prepare data
    let mut app = init_app!(1, 0);

    let dogs = generate_dogs(3);
    let dog_resources = ResourceDataWrapper::from_entities(&dogs, "http://localhost:1234/api/v1");
    for dog in &dog_resources {
        let req = post("/api/v1/dogs", &dog);
        let resp = call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    let dogs_idents =
        dog_resources.iter().map(|r| r.data.id.clone()).collect::<Vec<ResourceIdentifier>>();
    let (_first_dogs_idents, second_dogs_idents) = dogs_idents.split_first().unwrap();
    let (first_dogs, second_dogs) = dogs.split_first().unwrap();
    let first_dogs: Vec<Dog> = vec![first_dogs].into_iter().cloned().collect();
    let masters = vec![Human::from(first_dogs.as_slice()), second_dogs.into()];

    let master_resources =
        ResourceDataWrapper::from_entities(&masters, "http://localhost:1234/api/v1");
    for master in &master_resources {
        let req = post("/api/v1/people", &master);
        let resp = call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    // Delete all dogs from the second master
    let deleted_identifier_data =
        IdentifierDataWrapper { data: IdentifierData::Multiple(Vec::from(second_dogs_idents)) };
    let second_master_id = masters[1].id.to_string();
    let req = delete(
        format!("/api/v1/people/{}/relationships/dogs", second_master_id).as_str(),
        &deleted_identifier_data,
    );
    let resp = call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = get(format!("/api/v1/people/{}", second_master_id).as_str());
    let doc: Document = read_response_json(&mut app, req).await;
    let (second_human_resource, included) = doc.into_single().unwrap();
    assert!(included.is_empty());
    assert!(second_human_resource.relationships.get("dogs").unwrap().data.data().is_empty());

    // Add the removed pets to the first master
    let first_master_id = masters.first().unwrap().id.to_string();
    let req = post(
        format!("/api/v1/people/{}/relationships/dogs", first_master_id).as_str(),
        &deleted_identifier_data,
    );
    let resp = call_service(&mut app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = get(format!("/api/v1/people/{}", first_master_id).as_str());
    let doc: Document = read_response_json(&mut app, req).await;
    let (first_human_resource, included) = doc.into_single().unwrap();
    assert_eq!(included.len(), 3);

    let first_pets: Vec<ResourceIdentifier> =
        first_human_resource.relationships.get("dogs").unwrap().data.data();
    let first_pets_set: HashSet<ResourceIdentifier> = HashSet::from_iter(first_pets);
    assert_eq!(first_pets_set, HashSet::from_iter(dogs_idents));
}
