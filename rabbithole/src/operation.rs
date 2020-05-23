use crate::entity::SingleEntity;
use crate::model::document::Document;
use crate::model::relationship::Relationship;

use crate::model::link::{Links, RawUri};
use crate::model::resource::{IdentifierData, Resource};
use crate::model::{error, Meta};
use crate::query::Query;
use crate::Result;
use async_trait::async_trait;

pub type OperationResult<T> = Result<OperationResultData<T>>;
pub type CollectionResult<T> = Result<OperationResultData<Vec<T>>>;
pub type SingleResult<T> = Result<OperationResultData<Option<T>>>;
pub type UpdateResult<T> = Result<OperationResultData<(String, Option<T>)>>;

pub trait Operation {
    type Item: SingleEntity + Send + Sync;
}

#[derive(Default)]
pub struct OperationResultData<T: Default> {
    pub data: T,
    pub additional_links: Links,
    pub additional_meta: Meta,
}

#[async_trait]
pub trait Fetching: Operation {
    //    /// User defined `vec_to_document` function
    //    /// NOTICE:
    //    ///   - If using Page Query, it's *recommended* to:
    //    ///     - put `prev`, `next`, `first` and `last` into `links`
    //    ///     - put `totalPages` if `@type == PageBased`
    //    async fn vec_to_document(
    //        items: &[Self::Item], uri: &str, query: &Query, request_path: &RawUri,
    //    ) -> Result<Document> {
    //        Ok(items.to_document(uri, query, request_path)?)
    //    }
    /// Mapping to `/<ty>?<query>`
    #[allow(unused_variables)]
    async fn fetch_collection(
        &self, uri: &str, path: &RawUri, query: &Query,
    ) -> CollectionResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("fetch_collection", None))
    }
    /// Mapping to `/<ty>/<id>?<query>`
    #[allow(unused_variables)]
    async fn fetch_single(
        &self, id: &str, uri: &str, path: &RawUri, query: &Query,
    ) -> SingleResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("fetch_single", None))
    }
    /// Mapping to `/<ty>/<id>/relationships/<related_field>?<query>`
    #[allow(unused_variables)]
    async fn fetch_relationship(
        &self, id: &str, related_field: &str, uri: &str, path: &RawUri, query: &Query,
    ) -> OperationResult<Relationship> {
        Err(error::Error::OperationNotImplemented("fetch_relationship", None))
    }
    /// Mapping to `/<ty>/<id>/<related_field>?<query>`
    /// # Returns
    /// Because `rabbithole` can only get the field by String, so it cannot get the actual type, so you should return
    /// the document yourself
    #[allow(unused_variables)]
    async fn fetch_related(
        &self, id: &str, related_field: &str, uri: &str, path: &RawUri, query: &Query,
    ) -> Result<Document> {
        Err(error::Error::OperationNotImplemented("fetch_related", None))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceDataWrapper {
    pub data: Resource,
}

impl ResourceDataWrapper {
    pub fn from_entities<T>(entities: &[T], path: &str) -> Vec<Self>
    where
        T: SingleEntity,
    {
        entities
            .iter()
            .filter_map(|d| d.to_resource(&path, &Default::default()))
            .map(|data| Self { data })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentifierDataWrapper {
    pub data: IdentifierData,
}

#[async_trait]
pub trait Creating: Operation {
    /// Mapping to `POST /<ty>`
    /// # Returns
    ///
    /// If returns `Ok(Some(item))`, then will be mapped to `StatusCode == '201 Created'` with created Resource;
    /// If returns `Ok(None)`, then will be mapped to `StatusCode == '204 No Content'` with empty body
    #[allow(unused_variables)]
    async fn create(
        &mut self, data: &ResourceDataWrapper, uri: &str, path: &RawUri,
    ) -> SingleResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("create", None))
    }
}

#[async_trait]
pub trait Updating: Operation {
    /// Mapping to `PATCH /<ty>/<id>`
    /// # Returns
    ///
    /// If the result matches the incoming data, or in other words, the result matches the expectation of the user, then should return `None`, which will be mapped as `204 No Content`
    /// Otherwise, this function should return `200 OK`, with the whole updated resource
    #[allow(unused_variables)]
    async fn update_resource(
        &mut self, id: &str, data: &ResourceDataWrapper, uri: &str, path: &RawUri,
    ) -> SingleResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("update_resource", None))
    }
    /// Mapping to `PATCH /<ty>/<id>/relationships/<field>`
    /// # Arguments
    ///
    /// * `id_field` - The first string is the id of the resource, the second string is the field name of the relationship
    ///
    /// # Returns
    ///
    /// * A tuple of the updated result. The first string is the field name(should be equal with the second string of `id_field`)
    #[allow(unused_variables)]
    async fn replace_relationship(
        &mut self, id_field: &(String, String), data: &IdentifierDataWrapper, uri: &str,
        path: &RawUri,
    ) -> UpdateResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("replace_relationship", None))
    }
    /// Mapping to `POST /<ty>/<id>/relationships/<field>`
    /// # Arguments
    ///
    /// * `id_field` - The first string is the id of the resource, the second string is the field name of the relationship
    ///
    /// # Returns
    ///
    /// * A tuple of the updated result. The first string is the field name(should be equal with the second string of `id_field`)
    #[allow(unused_variables)]
    async fn add_relationship(
        &mut self, id_field: &(String, String), data: &IdentifierDataWrapper, uri: &str,
        path: &RawUri,
    ) -> UpdateResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("add_relationship", None))
    }
    /// Mapping to `DELETE /<ty>/<id>/relationships/<field>`
    /// # Arguments
    ///
    /// * `id_field` - The first string is the id of the resource, the second string is the field name of the relationship
    ///
    /// # Returns
    ///
    /// * A tuple of the updated result. The first string is the field name(should be equal with the second string of `id_field`)
    #[allow(unused_variables)]
    async fn remove_relationship(
        &mut self, id_field: &(String, String), data: &IdentifierDataWrapper, uri: &str,
        path: &RawUri,
    ) -> UpdateResult<Self::Item> {
        Err(error::Error::OperationNotImplemented("remove_relationship", None))
    }
}

#[async_trait]
pub trait Deleting: Operation {
    /// Mapping to `DELETE /<ty>/<id>`
    #[allow(unused_variables)]
    async fn delete_resource(&mut self, id: &str, uri: &str, path: &RawUri) -> OperationResult<()> {
        Err(error::Error::OperationNotImplemented("delete_resource", None))
    }
}
