extern crate proc_macro;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate lazy_static;

mod backend;
mod error;
mod field;

use crate::error::EntityDecoratorError;
use crate::field::{get_field_type, FieldType};
use proc_macro::TokenStream;
use quote::{quote, TokenStreamExt};
use std::collections::HashSet;
use syn::DeriveInput;

type FieldBundle<'a> =
    (&'a syn::Ident, Vec<&'a syn::Ident>, Vec<&'a syn::Ident>, Vec<&'a syn::Ident>);

#[proc_macro_derive(EntityDecorator, attributes(entity))]
pub fn derive(input: TokenStream) -> TokenStream {
    inner_derive(input).unwrap_or_else(|err| err.to_compile_error()).into()
}

#[allow(clippy::cognitive_complexity)]
fn inner_derive(input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let ast: DeriveInput = syn::parse(input)?;
    let decorated_struct: &syn::Ident = &ast.ident;
    let struct_lifetime = &ast.generics;

    let (entity_type, backends, service) = get_entity_type(&ast)?;

    let (id, attrs, to_ones, to_manys) = get_fields(&ast)?;

    let mut res = quote! {
        impl #struct_lifetime rabbithole::entity::Entity for #decorated_struct#struct_lifetime {
            fn included(&self, uri: &str,
                include_query: &std::option::Option<rabbithole::query::IncludeQuery>,
                fields_query: &rabbithole::query::FieldsQuery,
            ) -> rabbithole::Result<rabbithole::model::document::Included> {
                use rabbithole::entity::SingleEntity;
                use std::convert::TryInto;
                let mut included: rabbithole::model::document::Included = Default::default();

                if let Some(included_fields) = include_query {
                    for inc in included_fields {
                        if inc.contains('.') {
                            return Err(rabbithole::model::error::Error::RelationshipPathNotSupported(&inc, None));
                        }
                    }
                }
                #(
                    if let Some(included_fields) = include_query {
                        if included_fields.contains(stringify!(#to_ones)) {
                            if let Some(inc) = self.#to_ones.to_resource(uri, fields_query) {
                                included.insert(inc.id.clone(), inc);
                            }
                        }
                    } else {
                        if let Some(inc) = self.#to_ones.to_resource(uri, fields_query) {
                            included.insert(inc.id.clone(), inc);
                        }
                    }
                )*
                #(
                    if let Some(included_fields) = include_query {
                        if included_fields.contains(stringify!(#to_manys)) {
                            for item in &self.#to_manys {
                                if let Some(inc) = item.to_resource(uri, fields_query) {
                                    included.insert(inc.id.clone(), inc);
                                }
                            }
                        }
                    } else {
                        for item in &self.#to_manys {
                            if let Some(inc) = item.to_resource(uri, fields_query) {
                                included.insert(inc.id.clone(), inc);
                            }
                        }
                    }
                )*
                Ok(included)
             }

            fn to_document(&self, uri: &str, query: &rabbithole::query::Query, request_path: rabbithole::model::link::RawUri,  additional_links: rabbithole::model::link::Links, additional_meta: rabbithole::model::Meta,) -> rabbithole::Result<rabbithole::model::document::Document> {
                rabbithole::entity::SingleEntity::to_document(&self, uri, query, request_path, additional_links, additional_meta)
             }
        }

        impl #struct_lifetime rabbithole::entity::SingleEntity for #decorated_struct#struct_lifetime {
            fn ty() -> std::string::String { #entity_type.to_string() }
            fn id(&self) -> std::string::String { self.#id.to_string() }

            fn attributes(&self) -> rabbithole::model::resource::Attributes {
                let mut attr_map: std::collections::HashMap<String, serde_json::Value> = std::default::Default::default();
                #(  if let Ok(json_value) = serde_json::to_value(self.#attrs.clone()) { attr_map.insert(stringify!(#attrs).to_string(), json_value); } )*
                attr_map.into()
            }

            fn relationships(&self, uri: &str) -> rabbithole::model::relationship::Relationships {
                let mut relat_map: rabbithole::model::relationship::Relationships = std::default::Default::default();
                #(
                    if let Some(relat_id) = self.#to_ones.to_resource_identifier() {
                        let data = rabbithole::model::resource::IdentifierData::Single(Some(relat_id));
                        let relat = rabbithole::model::relationship::Relationship { data, links: self.to_relationship_links(stringify!(#to_ones), uri), ..std::default::Default::default() };
                        relat_map.insert(stringify!(#to_ones).to_string(), relat);
                    }
                )*

                #(
                    let mut relat_ids: rabbithole::model::resource::ResourceIdentifiers = std::default::Default::default();
                    for item in &self.#to_manys {
                        if let Some(relat_id) = item.to_resource_identifier() {
                            relat_ids.push(relat_id);
                        }
                    }
                    let data = rabbithole::model::resource::IdentifierData::Multiple(relat_ids);
                    let relat = rabbithole::model::relationship::Relationship { data, links: self.to_relationship_links(stringify!(#to_manys), uri), ..std::default::Default::default() };
                    relat_map.insert(stringify!(#to_manys).to_string(), relat);
                )*

                relat_map
            }
        }


    };

    for back in backends {
        if back == "actix" {
            res.append_all(vec![backend::actix::generate_app(&service, &entity_type)]);
        }
    }

    Ok(res)
}

fn get_meta(attrs: &[syn::Attribute]) -> syn::Result<Vec<syn::Meta>> {
    Ok(attrs
        .iter()
        .filter(|a| a.path.is_ident("entity"))
        .filter_map(|a| {
            let res = a.parse_meta();
            res.ok()
        })
        .collect::<Vec<syn::Meta>>())
}

fn get_entity_type(ast: &syn::DeriveInput) -> syn::Result<(String, HashSet<String>, syn::Path)> {
    let mut ty_opt: Option<String> = None;
    let mut backends: HashSet<String> = Default::default();
    let mut service: Option<syn::Path> = None;

    for meta in get_meta(&ast.attrs)? {
        if let syn::Meta::List(syn::MetaList { ref nested, .. }) = meta {
            if let Some(syn::NestedMeta::Meta(ref meta_item)) = nested.last() {
                match meta_item {
                    syn::Meta::NameValue(syn::MetaNameValue {
                        path,
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) => match path.segments.last() {
                        Some(syn::PathSegment { ident, .. }) if ident == "type" => {
                            ty_opt = Some(lit_str.value());
                        },
                        _ => {},
                    },
                    syn::Meta::List(syn::MetaList { path, nested, .. }) => {
                        match path.segments.last() {
                            Some(syn::PathSegment { ident, .. }) if ident == "backend" => {
                                for nested_backend in nested {
                                    if let syn::NestedMeta::Meta(syn::Meta::Path(backend_path)) =
                                        nested_backend
                                    {
                                        if let Some(syn::PathSegment { ident, .. }) =
                                            backend_path.segments.last()
                                        {
                                            backends.insert(ident.to_string());
                                        }
                                    }
                                }
                            },
                            Some(syn::PathSegment { ident, .. }) if ident == "service" => {
                                for nested_service in nested {
                                    if let syn::NestedMeta::Meta(syn::Meta::Path(service_path)) =
                                        nested_service
                                    {
                                        service = Some(service_path.clone());
                                    }
                                }
                            },
                            _ => {},
                        }
                    },
                    _ => {},
                }
            }
        }
    }

    if ty_opt.is_none() {
        return Err(syn::Error::new_spanned(ast, EntityDecoratorError::InvalidEntityType));
    }

    if service.is_none() {
        return Err(syn::Error::new_spanned(ast, EntityDecoratorError::LackOfService));
    }

    Ok((ty_opt.unwrap(), backends, service.unwrap()))
}

fn get_fields(ast: &syn::DeriveInput) -> syn::Result<FieldBundle> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        let mut id = None;
        let mut attrs = vec![];
        let mut to_ones = vec![];
        let mut to_manys = vec![];

        for n in named {
            let f: FieldType = get_field_type(n)?;
            match (f, n.ident.as_ref()) {
                (FieldType::Id, Some(ident)) if id.is_none() => id = Some(ident),
                (FieldType::Id, _) => {
                    return Err(syn::Error::new_spanned(n, EntityDecoratorError::DuplicatedId))
                },
                (FieldType::ToOne, Some(ident)) => to_ones.push(ident),
                (FieldType::ToMany, Some(ident)) => to_manys.push(ident),
                (FieldType::Plain, Some(ident)) => {
                    attrs.push(ident);
                },
                _ => {
                    return Err(syn::Error::new_spanned(n, EntityDecoratorError::FieldWithoutName))
                },
            }
        }

        if let Some(id) = id {
            return Ok((id, attrs, to_ones, to_manys));
        }
    }
    Err(syn::Error::new_spanned(&ast.ident, EntityDecoratorError::InvalidEntityType))
}
