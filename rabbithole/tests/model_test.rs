mod helper;

use crate::helper::read_json_file;
use rabbithole::model::document::{Document, DocumentItem, PrimaryDataItem};
use rabbithole::model::error::Error;
use rabbithole::model::link::Links;
use rabbithole::model::resource::*;
use rabbithole::model::*;

use std::collections::HashSet;
use std::iter::FromIterator;

#[test]
fn error_from_json_string() {
    let _ = env_logger::try_init();

    let serialized = r#"
        {"id":"1", "links" : {}, "status" : "unknown", "code" : "code1", "title" : "error-title", "detail": "error-detail"}
        "#;

    let result: Result<Error, serde_json::Error> = serde_json::from_str(serialized);

    if let Err(err) = result {
        unreachable!("err: {:?}", err);
    }

    match result {
        Ok(jsonapierror) => match jsonapierror.id {
            Some(id) => assert_eq!(id, "1"),
            None => unreachable!(),
        },
        Err(err) => unreachable!("get err: {:?}", err),
    }
}

#[test]
fn single_resource_from_json_string() {
    let _ = env_logger::try_init();

    let serialized =
        r#"{ "id" :"1", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} }"#;
    let data: Result<Resource, serde_json::Error> = serde_json::from_str(serialized);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn multiple_resource_from_json_string() {
    let _ = env_logger::try_init();
    let serialized = r#"[
            { "id" :"1", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} },
            { "id" :"2", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} },
            { "id" :"3", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} }
        ]"#;
    let data: Result<Vec<Resource>, serde_json::Error> = serde_json::from_str(serialized);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn no_data_document_from_json_string() {
    let _ = env_logger::try_init();
    let serialized = r#"{
            "data" : null
        }"#;
    let data: Result<Document, serde_json::Error> = serde_json::from_str(serialized);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn single_data_document_from_json_string() {
    let _ = env_logger::try_init();
    let serialized = r#"{
            "data" : {
                "id" :"1", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {}
            }
        }"#;
    let data: Result<Document, serde_json::Error> = serde_json::from_str(serialized);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn multiple_data_document_from_json_string() {
    let _ = env_logger::try_init();
    let serialized = r#"{
            "data" : [
                { "id" :"1", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} },
                { "id" :"2", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} },
                { "id" :"3", "type" : "post", "attributes" : {}, "relationships" : {}, "links" : {} }
            ]
        }"#;
    let data: Result<Document, serde_json::Error> = serde_json::from_str(serialized);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn api_document_from_json_file() {
    let _ = env_logger::try_init();

    let s = crate::read_json_file("data/results.json");
    let data: Result<Document, serde_json::Error> = serde_json::from_str(&s);

    match data {
        Ok(res) => match res.item {
            DocumentItem::PrimaryData(Some((PrimaryDataItem::Multiple(arr), _))) => {
                assert_eq!(arr.len(), 1);
            },
            DocumentItem::PrimaryData(Some((PrimaryDataItem::Single(_), _))) => {
                unreachable!(
                    "api_document_from_json_file : Expected one Resource in a vector, not a \
                     direct Resource"
                );
            },
            DocumentItem::PrimaryData(None) => {
                unreachable!("api_document_from_json_file : Expected one Resource in a vector");
            },
            _ => unreachable!(),
        },
        Err(err) => {
            unreachable!("api_document_from_json_file : Error: {:?}", err);
        },
    }
}

#[test]
fn api_document_collection_from_json_file() {
    let _ = env_logger::try_init();

    let s = crate::read_json_file("data/collection.json");
    let data: Result<Document, serde_json::Error> = serde_json::from_str(&s);

    match data {
        Ok(res) => {
            match res.item {
                DocumentItem::PrimaryData(Some((PrimaryDataItem::Multiple(arr), included))) => {
                    assert_eq!(arr.len(), 1);

                    assert_eq!(included.len(), 3);
                    let ids: HashSet<&str> =
                        included.iter().map(|(inc_id, _)| inc_id.id.as_str()).collect();
                    assert_eq!(ids, HashSet::from_iter(vec!["9", "5", "12"]));
                },
                DocumentItem::PrimaryData(Some((PrimaryDataItem::Single(_), _))) => unreachable!(
                    "api_document_collection_from_json_file : Expected one Resource in a vector, \
                     not a direct Resource"
                ),

                DocumentItem::PrimaryData(None) => unreachable!(
                    "api_document_collection_from_json_file : Expected one Resource in a vector"
                ),

                _ => unreachable!(),
            }
            assert_eq!(res.links.len(), 3);
        },
        Err(err) => {
            unreachable!("api_document_collection_from_json_file : Error: {:?}", err);
        },
    }
}

#[test]
fn can_deserialize_jsonapi_example_resource_001() {
    let _ = env_logger::try_init();

    let s = crate::read_json_file("data/resource_001.json");
    let data: Result<Resource, serde_json::Error> = serde_json::from_str(&s);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn can_deserialize_jsonapi_example_resource_002() {
    let _ = env_logger::try_init();
    let s = crate::read_json_file("data/resource_002.json");
    let data: Result<Resource, serde_json::Error> = serde_json::from_str(&s);

    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn can_deserialize_jsonapi_example_resource_003() {
    let _ = env_logger::try_init();
    let s = crate::read_json_file("data/resource_003.json");
    let data: Result<Resource, serde_json::Error> = serde_json::from_str(&s);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn can_deserialize_jsonapi_example_compound_document() {
    let _ = env_logger::try_init();
    let s = crate::read_json_file("data/compound_document.json");
    let data: Result<Document, serde_json::Error> = serde_json::from_str(&s);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn can_deserialize_jsonapi_example_links_001() {
    let _ = env_logger::try_init();
    let s = crate::read_json_file("data/links_001.json");
    let data: Result<Links, serde_json::Error> = serde_json::from_str(&s);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn can_deserialize_jsonapi_example_links_002() {
    let _ = env_logger::try_init();
    let s = crate::read_json_file("data/links_002.json");
    let data: Result<Links, serde_json::Error> = serde_json::from_str(&s);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn can_deserialize_jsonapi_example_jsonapi_info() {
    let _ = env_logger::try_init();
    let s = crate::read_json_file("data/jsonapi_info_001.json");
    let data: Result<JsonApiInfo, serde_json::Error> = serde_json::from_str(&s);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}

#[test]
fn it_omits_empty_document_and_primary_data_keys() {
    let _ = env_logger::try_init();

    let resource = Resource { id: ResourceIdentifier::new("test", "123"), ..Default::default() };
    let doc = Document {
        item: DocumentItem::PrimaryData(Some((
            PrimaryDataItem::Single(Box::new(resource)),
            Default::default(),
        ))),
        ..Default::default()
    };

    assert_eq!(serde_json::to_string(&doc).unwrap(), r#"{"data":{"type":"test","id":"123"}}"#);
}

#[test]
fn it_does_not_omit_an_empty_primary_data() {
    let doc = Document { item: DocumentItem::PrimaryData(None), ..Default::default() };

    assert_eq!(serde_json::to_string(&doc).unwrap(), r#"{"data":null}"#);
}

#[test]
fn it_omits_empty_error_keys() {
    let error = Error { id: Some("error_id".to_string()), ..Default::default() };
    let doc = Document { item: DocumentItem::Errors(vec![error]), ..Default::default() };
    assert_eq!(serde_json::to_string(&doc).unwrap(), r#"{"errors":[{"id":"error_id"}]}"#);
}

#[test]
fn it_allows_for_optional_attributes() {
    let _ = env_logger::try_init();
    let serialized = r#"{
            "data" : {
                "id" :"1", "type" : "post", "relationships" : {}, "links" : {}
            }
        }"#;
    let data: Result<Document, serde_json::Error> = serde_json::from_str(serialized);
    if let Err(err) = data {
        unreachable!("err: {:?}", err);
    }
}
