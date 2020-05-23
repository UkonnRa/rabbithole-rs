use async_trait::async_trait;

use rabbithole::operation::*;

use rabbithole::model::error::Error;

use crate::model::dog::Dog;
use crate::model::human::Human;
use crate::service::dog::DogService;
use crate::service::*;
use futures::lock::Mutex;
use rabbithole::entity::{Entity, SingleEntity};
use rabbithole::model::document::Document;
use rabbithole::model::error;

use rabbithole::model::relationship::Relationship;
use rabbithole::model::resource::{AttributeField, IdentifierData, ResourceIdentifier};
use rabbithole::query::Query;

use rabbithole::model::link::RawUri;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct HumanService(HashMap<String, Human>, Arc<Mutex<DogService>>);
impl HumanService {
    pub fn new(dog_service: Arc<Mutex<DogService>>) -> Arc<Mutex<HumanService>> {
        Arc::new(Mutex::new(Self(Default::default(), dog_service)))
    }
}

impl Operation for HumanService {
    type Item = Human;
}

#[async_trait]
impl Fetching for HumanService {
    async fn fetch_collection(
        &self, uri: &str, path: &RawUri, query: &Query,
    ) -> CollectionResult<Human> {
        let data: Vec<Human> = self.0.values().cloned().collect();
        let (data, links) = query.query(data, uri, path)?;
        Ok(OperationResultData { data, additional_links: links, ..Default::default() })
    }

    async fn fetch_single(
        &self, id: &str, _uri: &str, _path: &RawUri, _query: &Query,
    ) -> SingleResult<Human> {
        Ok(OperationResultData { data: self.0.get(id).map(Clone::clone), ..Default::default() })
    }

    async fn fetch_relationship(
        &self, id: &str, related_field: &str, uri: &str, _path: &RawUri, query: &Query,
    ) -> OperationResult<Relationship> {
        if let Some(human) = self.0.get(id) {
            let resource = human.to_resource(&uri.to_string(), &query.fields).unwrap();
            if let Some(relat) = resource.relationships.get(related_field) {
                Ok(OperationResultData { data: relat.clone(), ..Default::default() })
            } else {
                Err(error::Error::FieldNotExist(related_field, None))
            }
        } else {
            Err(ENTITY_NOT_FOUND.clone())
        }
    }

    async fn fetch_related(
        &self, id: &str, related_field: &str, uri: &str, path: &RawUri, query: &Query,
    ) -> Result<Document, Error> {
        if let Some(human) = self.0.get(id) {
            if related_field == "dogs" {
                Ok(human.dogs.to_document(
                    &uri.to_string(),
                    query,
                    path.clone(),
                    Default::default(),
                    Default::default(),
                )?)
            } else {
                Err(error::Error::FieldNotExist(related_field, None))
            }
        } else {
            Err(ENTITY_NOT_FOUND.clone())
        }
    }
}

#[async_trait]
impl Creating for HumanService {
    async fn create(
        &mut self, data: &ResourceDataWrapper, _uri: &str, _path: &RawUri,
    ) -> SingleResult<Human> {
        let ResourceDataWrapper { data } = data;
        let id = if !data.id.id.is_empty() {
            if self.0.contains_key(&data.id.id) {
                Err(DUPLICATE_ID.clone())
            } else {
                Uuid::parse_str(&data.id.id).map_err(|_| INVALID_UUID.clone())
            }
        } else {
            Ok(Uuid::new_v4())
        }?;

        let dog_ids: Vec<ResourceIdentifier> =
            data.relationships.get("dogs").map(|r| r.data.data()).unwrap_or_default();
        let dog_ids: Vec<String> = dog_ids.iter().map(|id| id.id.clone()).collect();
        let dogs: Vec<Dog> = self.1.lock().await.get_by_ids(&dog_ids)?;

        if let AttributeField(serde_json::Value::String(name)) =
            data.attributes.get_field("name")?
        {
            let human = Human { id, name: name.clone(), dogs };
            self.0.insert(human.id.clone().to_string(), human.clone());
            Ok(OperationResultData { data: Some(human), ..Default::default() })
        } else {
            Err(WRONG_FIELD_TYPE.clone())
        }
    }
}

#[async_trait]
impl Updating for HumanService {
    async fn update_resource(
        &mut self, id: &str, data: &ResourceDataWrapper, _uri: &str, _path: &RawUri,
    ) -> SingleResult<Human> {
        if let Some(mut human) = self.0.get(id).cloned() {
            let new_attrs = &data.data.attributes;
            let new_relats = &data.data.relationships;
            if let Some(dog_ids) = new_relats.get("dogs").map(|r| r.data.data()) {
                let dog_ids: Vec<String> =
                    dog_ids.iter().map(|r: &ResourceIdentifier| r.id.clone()).collect();
                let dogs = self.1.clone().lock().await.get_by_ids(&dog_ids)?;
                human.dogs = dogs;
            }
            if let Ok(AttributeField(serde_json::Value::String(name))) = new_attrs.get_field("name")
            {
                human.name = name.clone();
            }
            self.0.insert(id.to_string(), human);
            Ok(OperationResultData { data: None, ..Default::default() })
        } else {
            Err(ENTITY_NOT_FOUND.clone())
        }
    }

    async fn replace_relationship(
        &mut self, id_field: &(String, String), data: &IdentifierDataWrapper, _uri: &str,
        _path: &RawUri,
    ) -> UpdateResult<Human> {
        let (id, field) = id_field;
        if let Some(human) = self.0.get_mut(id) {
            let IdentifierDataWrapper { data } = data;
            match data {
                IdentifierData::Single(_) => Err(MULTIPLE_RELATIONSHIP_NEEDED.clone()),
                IdentifierData::Multiple(datas) => {
                    let ids: Vec<String> = datas
                        .iter()
                        .filter_map(|i| if &i.ty == field { Some(i.id.clone()) } else { None })
                        .collect();
                    let dogs = self.1.lock().await.get_by_ids(&ids)?;
                    human.dogs = dogs;
                    Ok(OperationResultData { data: (field.clone(), None), ..Default::default() })
                },
            }
        } else {
            Err(ENTITY_NOT_FOUND.clone())
        }
    }

    async fn add_relationship(
        &mut self, id_field: &(String, String), data: &IdentifierDataWrapper, _uri: &str,
        _path: &RawUri,
    ) -> UpdateResult<Human> {
        let (id, field) = id_field;
        if let Some(human) = self.0.get_mut(id) {
            let IdentifierDataWrapper { data } = data;
            match data {
                IdentifierData::Single(_) => Err(MULTIPLE_RELATIONSHIP_NEEDED.clone()),
                IdentifierData::Multiple(datas) => {
                    let ids: Vec<String> = datas
                        .iter()
                        .filter_map(|i| if &i.ty == field { Some(i.id.clone()) } else { None })
                        .collect();
                    let mut dogs = self.1.lock().await.get_by_ids(&ids)?;
                    human.add_dogs(&mut dogs);
                    Ok(OperationResultData { data: (field.clone(), None), ..Default::default() })
                },
            }
        } else {
            Err(ENTITY_NOT_FOUND.clone())
        }
    }

    async fn remove_relationship(
        &mut self, id_field: &(String, String), data: &IdentifierDataWrapper, _uri: &str,
        _path: &RawUri,
    ) -> UpdateResult<Human> {
        let (id, field) = id_field;
        if let Some(human) = self.0.get_mut(id) {
            let IdentifierDataWrapper { data } = data;
            match data {
                IdentifierData::Single(_) => Err(MULTIPLE_RELATIONSHIP_NEEDED.clone()),
                IdentifierData::Multiple(datas) => {
                    let ids: Vec<String> = datas
                        .iter()
                        .filter_map(|i| if &i.ty == field { Some(i.id.clone()) } else { None })
                        .collect();
                    human.remove_dogs(&ids);
                    Ok(OperationResultData { data: (field.clone(), None), ..Default::default() })
                },
            }
        } else {
            Err(ENTITY_NOT_FOUND.clone())
        }
    }
}

#[async_trait]
impl Deleting for HumanService {
    async fn delete_resource(
        &mut self, id: &str, _uri: &str, _path: &RawUri,
    ) -> OperationResult<()> {
        self.0.remove(id);
        Ok(OperationResultData { data: (), ..Default::default() })
    }
}
