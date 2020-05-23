use crate::model::document::{Document, Included};
use crate::model::link::{Link, Links, RawUri};
use crate::model::relationship::{RelationshipLinks, Relationships};
use crate::model::resource::{Attributes, Resource, ResourceIdentifier};
use crate::model::Meta;
use crate::query::*;
use crate::Result;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::ops::Deref;
pub trait Entity: Serialize + Clone {
    /// Returns the `included` field of this entity
    ///
    /// `include_query`: If exists, only the `included` item whose `ty` is in the `include_query`
    ///                  will be retained
    ///                  If exists but empty, means all `included` fields will be ignored
    ///                  If not exists, all the `included` fields will be retained   
    ///
    /// `fields_query`: For any resources whose `ty` is in the `fields_query`, their `relationship`
    ///                 and `attributes` will be filtered. Only the field name inside the `field_query`
    ///                 item will be retained
    #[doc(hidden)]
    fn included(
        &self, uri: &str, include_query: &Option<IncludeQuery>, fields_query: &FieldsQuery,
    ) -> Result<Included>;

    /// Returns a `Document` based on `query`. This function will do all of the actions databases should do in memory,
    /// using a trivial iter way. But I still recommend you guys implement `to_document` or `to_document_async` yourself
    /// for better performance
    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document>;
}

pub trait SingleEntity: Entity {
    #[doc(hidden)]
    fn ty() -> String;
    #[doc(hidden)]
    fn id(&self) -> String;
    #[doc(hidden)]
    fn attributes(&self) -> Attributes;
    #[doc(hidden)]
    fn relationships(&self, uri: &str) -> Relationships;

    #[doc(hidden)]
    fn links(&self, uri: &str) -> Links {
        let slf = format!(
            "{uri}/{ty}/{id}",
            uri = uri,
            ty = <Self as SingleEntity>::ty(),
            id = self.id()
        )
        .parse::<Link>()
        .unwrap();
        HashMap::from_iter(vec![("self".into(), slf)])
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, mut additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        let (key, value) = Link::slf(uri, request_path);
        additional_links.insert(key, value);
        let mut doc = Document::single_resource(
            self.to_resource(uri, &query.fields).unwrap(),
            self.included(uri, &query.include, &query.fields)?,
        );
        doc.extend_links(additional_links);
        doc.extend_meta(additional_meta);
        Ok(doc)
    }

    fn to_resource_identifier(&self) -> Option<ResourceIdentifier> {
        Some(ResourceIdentifier { ty: <Self as SingleEntity>::ty(), id: self.id() })
    }

    fn to_resource(&self, uri: &str, fields_query: &FieldsQuery) -> Option<Resource> {
        let mut attributes = self.attributes();
        let mut relationships = self.relationships(uri);
        for (k, vs) in fields_query.iter() {
            if &<Self as SingleEntity>::ty() == k {
                attributes = attributes.retain(vs);
                relationships.retain(|k, _| vs.contains(k));
            }
        }

        Some(Resource {
            id: ResourceIdentifier { id: self.id(), ty: Self::ty() },
            attributes,
            relationships,
            links: self.links(uri),
            ..Default::default()
        })
    }

    fn to_relationship_links(&self, field_name: &str, uri: &str) -> RelationshipLinks {
        let slf = format!(
            "{uri}/{ty}/{id}/relationships/{field_name}",
            uri = uri,
            ty = <Self as SingleEntity>::ty(),
            id = self.id(),
            field_name = field_name
        );
        let slf = slf.parse::<Link>().unwrap();
        let related = format!(
            "{uri}/{ty}/{id}/{field_name}",
            uri = uri,
            ty = <Self as SingleEntity>::ty(),
            id = self.id(),
            field_name = field_name
        );
        let related = related.parse::<Link>().unwrap();

        HashMap::from_iter(vec![("self".into(), slf), ("related".into(), related)]).into()
    }

    fn cmp_field(&self, field: &str, other: &Self) -> Result<Ordering> {
        self.attributes().cmp(field, &other.attributes())
    }
}

impl<T: SingleEntity> SingleEntity for Option<T> {
    fn ty() -> String { T::ty() }

    fn id(&self) -> String { self.as_ref().map(SingleEntity::id).unwrap() }

    fn attributes(&self) -> Attributes { self.as_ref().map(SingleEntity::attributes).unwrap() }

    fn relationships(&self, uri: &str) -> Relationships {
        self.as_ref().map(|op| op.relationships(uri)).unwrap()
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        if let Some(item) = self {
            SingleEntity::to_document(
                item,
                uri,
                query,
                request_path,
                additional_links,
                additional_meta,
            )
        } else {
            Ok(Document::default())
        }
    }

    fn to_resource_identifier(&self) -> Option<ResourceIdentifier> {
        self.as_ref().and_then(SingleEntity::to_resource_identifier)
    }

    fn to_resource(&self, uri: &str, query: &FieldsQuery) -> Option<Resource> {
        self.as_ref().and_then(|e| e.to_resource(uri, query))
    }
}

impl<T: Entity> Entity for Option<T> {
    fn included(
        &self, uri: &str, include_query: &Option<IncludeQuery>, fields_query: &FieldsQuery,
    ) -> Result<Included> {
        if let Some(s) = self {
            s.included(uri, include_query, fields_query)
        } else {
            Ok(Default::default())
        }
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        self.as_ref()
            .map(|op| op.to_document(uri, query, request_path, additional_links, additional_meta))
            .unwrap()
    }
}

impl<T: SingleEntity> SingleEntity for Box<T> {
    fn ty() -> String { T::ty() }

    fn id(&self) -> String { self.as_ref().id() }

    fn attributes(&self) -> Attributes { self.as_ref().attributes() }

    fn relationships(&self, uri: &str) -> Relationships { self.as_ref().relationships(uri) }
}

impl<T: Entity> Entity for Box<T> {
    fn included(
        &self, uri: &str, include_query: &Option<IncludeQuery>, fields_query: &FieldsQuery,
    ) -> Result<Included> {
        self.as_ref().included(uri, include_query, fields_query)
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        self.as_ref().to_document(uri, query, request_path, additional_links, additional_meta)
    }
}

impl<T: SingleEntity> SingleEntity for &T
where
    Self: Clone,
{
    fn ty() -> String { T::ty() }

    fn id(&self) -> String { self.deref().id() }

    fn attributes(&self) -> Attributes { self.deref().attributes() }

    fn relationships(&self, uri: &str) -> Relationships { self.deref().relationships(uri) }
}

impl<T: Entity> Entity for &T
where
    Self: Clone,
{
    fn included(
        &self, uri: &str, include_query: &Option<IncludeQuery>, fields_query: &FieldsQuery,
    ) -> Result<Included> {
        self.deref().included(uri, include_query, fields_query)
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        self.deref().to_document(uri, query, request_path, additional_links, additional_meta)
    }
}

impl<T: SingleEntity> Entity for &[T] {
    fn included(
        &self, uri: &str, include_query: &Option<IncludeQuery>, fields_query: &FieldsQuery,
    ) -> Result<Included> {
        let includes: Vec<Included> = self
            .iter()
            .map(|e| e.included(uri, include_query, fields_query))
            .collect::<Result<Vec<Included>>>()?;
        Ok(includes.into_iter().flat_map(|s| s.into_iter()).collect())
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, mut additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        let entities = self.to_vec();
        let (key, value) = Link::slf(uri, request_path);
        let resources = entities.iter().filter_map(|e| e.to_resource(uri, &query.fields)).collect();
        additional_links.insert(key, value);
        let mut doc = Document::multiple_resources(
            resources,
            self.included(uri, &query.include, &query.fields)?,
        );
        doc.extend_links(additional_links);
        doc.extend_meta(additional_meta);

        Ok(doc)
    }
}

impl<T: SingleEntity> Entity for Vec<T> {
    fn included(
        &self, uri: &str, include_query: &Option<IncludeQuery>, fields_query: &FieldsQuery,
    ) -> Result<Included> {
        self.as_slice().included(uri, include_query, fields_query)
    }

    fn to_document(
        &self, uri: &str, query: &Query, request_path: RawUri, additional_links: Links,
        additional_meta: Meta,
    ) -> Result<Document> {
        self.as_slice().to_document(uri, query, request_path, additional_links, additional_meta)
    }
}
