pub mod settings;

use crate::settings::JsonApiSettings;
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::{header, HeaderMap, StatusCode};
use actix_web::web;
use actix_web::{HttpRequest, HttpResponse};
use futures::lock::Mutex;
use rabbithole::entity::{Entity, SingleEntity};
use rabbithole::model::document::Document;
use rabbithole::model::error;
use rabbithole::model::version::JsonApiVersion;
use rabbithole::operation::{
    Creating, Deleting, Fetching, IdentifierDataWrapper, OperationResultData, ResourceDataWrapper,
};
use rabbithole::query::QuerySettings;
use rabbithole::rule::RuleDispatcher;
use rabbithole::JSON_API_HEADER;
use serde::Deserialize;
use std::sync::Arc;

fn error_to_response(err: error::Error) -> HttpResponse {
    new_json_api_resp(
        err.status.as_deref().and_then(|s| s.parse().ok()).unwrap_or(StatusCode::BAD_REQUEST),
    )
    .json(err)
}

#[derive(Deserialize, Debug, Clone)]
pub struct ActixSettings {
    pub host: String,
    pub path: String,
    pub port: u32,
    pub jsonapi: JsonApiSettings,
    pub query: QuerySettings,
}

macro_rules! to_response {
    (Resource: $this:ident, $item:ident) => {{
        let rabbithole::operation::OperationResultData { data, additional_links, additional_meta } =
            $item;
        let resource = data.to_resource(&$this.uri().to_string(), &Default::default());
        match resource {
            Some(mut resource) => {
                resource.extend_meta(additional_meta);
                resource.extend_links(additional_links);
                Ok(actix_web::HttpResponse::Ok()
                    .json(rabbithole::operation::ResourceDataWrapper { data: resource }))
            },
            None => Ok(actix_web::HttpResponse::NoContent().finish()),
        }
    }};
    (Relationship: $this:ident, $item:ident) => {{
        let rabbithole::operation::OperationResultData { data, additional_links, additional_meta } =
            $item;
        let (field_name, item) = data;
        let resource = item.to_resource(&$this.uri().to_string(), &Default::default());
        match resource {
            Some(mut resource) => {
                resource.extend_meta(additional_meta);
                resource.extend_links(additional_links);

                Ok(actix_web::HttpResponse::Ok().json(
                    rabbithole::operation::IdentifierDataWrapper {
                        data: resource.relationships.get(&field_name).cloned().unwrap().data,
                    },
                ))
            },
            None => Ok(actix_web::HttpResponse::NoContent().finish()),
        }
    }};
}

macro_rules! single_step_operation {
    ($return_ty:ident:  $fn_name:ident, $mark:ident, $( $param:ident => $ty:ty ),+) => {
        pub async fn $fn_name<T>(this: web::Data<Self>, service: actix_web::web::Data<std::sync::Arc<futures::lock::Mutex<T>>>, req: actix_web::HttpRequest, $($param: $ty),+)
          -> Result<actix_web::HttpResponse, actix_web::Error>
            where
                T: 'static + rabbithole::operation::$mark + Send + Sync,
                T::Item: rabbithole::entity::SingleEntity + Send + Sync,
          {
            if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
                return Ok(err_resp);
            }
            let uri = &this.uri().to_string();
            let path = req.uri().clone().into();

            match service.lock().await.$fn_name($(&$param.into_inner()),+, uri, &path).await {
                Ok(item) => {
                    to_response!($return_ty: this, item)
                },
                Err(err) => Ok(error_to_response(err)),
            }
        }
    };
}

impl ActixSettings {
    single_step_operation!(Resource: update_resource, Updating, params => web::Path<String>, body => web::Json<ResourceDataWrapper>);

    single_step_operation!(Relationship: replace_relationship, Updating, params => web::Path<(String, String)>, body => web::Json<IdentifierDataWrapper>);

    single_step_operation!(Relationship: add_relationship, Updating, params => web::Path<(String, String)>, body => web::Json<IdentifierDataWrapper>);

    single_step_operation!(Relationship: remove_relationship, Updating, params => web::Path<(String, String)>, body => web::Json<IdentifierDataWrapper>);

    fn uri(&self) -> url::Url {
        let uri = format!("http://{}:{}", self.host, self.port).parse::<url::Url>().unwrap();
        uri.join(&self.path).unwrap()
    }

    pub async fn delete_resource<T>(
        this: web::Data<Self>, service: web::Data<Arc<Mutex<T>>>, params: web::Path<String>,
        req: actix_web::HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        T: 'static + Deleting + Send + Sync,
        T::Item: rabbithole::entity::SingleEntity + Send + Sync,
    {
        if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
            return Ok(err_resp);
        }

        let uri = &this.uri().to_string();
        let path = req.uri().clone().into();

        match service.lock().await.delete_resource(&params.into_inner(), uri, &path).await {
            Ok(OperationResultData { additional_links, additional_meta, .. }) => {
                if additional_links.is_empty() && additional_meta.is_empty() {
                    Ok(actix_web::HttpResponse::NoContent().finish())
                } else {
                    Ok(actix_web::HttpResponse::Ok()
                        .json(Document::null(additional_links, additional_meta)))
                }
            },
            Err(err) => Ok(error_to_response(err)),
        }
    }

    pub async fn create<T>(
        this: web::Data<Self>, service: web::Data<Arc<Mutex<T>>>, req: actix_web::HttpRequest,
        body: web::Json<ResourceDataWrapper>,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        T: 'static + Creating + Send + Sync,
        T::Item: rabbithole::entity::SingleEntity + Send + Sync,
    {
        if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
            return Ok(err_resp);
        }

        let uri = &this.uri().to_string();
        let path = req.uri().clone().into();

        match service.lock().await.create(&body.into_inner(), uri, &path).await {
            Ok(OperationResultData { data, additional_links, additional_meta }) => {
                let mut resource = data.to_resource(uri, &Default::default()).unwrap();
                resource.extend_meta(additional_meta);
                resource.extend_links(additional_links);
                Ok(actix_web::HttpResponse::Ok().json(ResourceDataWrapper { data: resource }))
            },
            Err(err) => Ok(error_to_response(err)),
        }
    }

    pub async fn fetch_collection<T>(
        this: web::Data<Self>, service: web::Data<Arc<Mutex<T>>>, req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        T: 'static + Fetching + Send + Sync,
        T::Item: rabbithole::entity::SingleEntity + Send + Sync,
    {
        if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
            return Ok(err_resp);
        }

        let uri = &this.uri().to_string();
        let path: rabbithole::model::link::RawUri = req.uri().clone().into();

        match this.query.decode_path(&path) {
            Ok(query) => match service.lock().await.fetch_collection(uri, &path, &query).await {
                Ok(OperationResultData { data, additional_links, additional_meta }) => {
                    match data.to_document(uri, &query, path, additional_links, additional_meta) {
                        Ok(doc) => Ok(HttpResponse::Ok().json(doc)),
                        Err(err) => Ok(error_to_response(err)),
                    }
                },
                Err(err) => Ok(error_to_response(err)),
            },
            Err(err) => Ok(error_to_response(err)),
        }
    }

    pub async fn fetch_single<T>(
        this: web::Data<Self>, service: web::Data<Arc<Mutex<T>>>, param: web::Path<String>,
        req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        T: 'static + Fetching + Send + Sync,
        T::Item: rabbithole::entity::SingleEntity + Send + Sync,
    {
        if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
            return Ok(err_resp);
        }

        let uri = &this.uri().to_string();
        let path = req.uri().clone().into();

        match this.query.decode_path(&path) {
            Ok(query) => {
                match service
                    .lock()
                    .await
                    .fetch_single(&param.into_inner(), uri, &path, &query)
                    .await
                {
                    Ok(OperationResultData { data, additional_links, additional_meta }) => {
                        match SingleEntity::to_document(
                            &data,
                            &this.uri().to_string(),
                            &query,
                            req.uri().into(),
                            additional_links,
                            additional_meta,
                        ) {
                            Ok(doc) => Ok(new_json_api_resp(StatusCode::OK).json(doc)),
                            Err(err) => Ok(error_to_response(err)),
                        }
                    },
                    Err(err) => Ok(error_to_response(err)),
                }
            },
            Err(err) => Ok(error_to_response(err)),
        }
    }

    pub async fn fetch_relationship<T>(
        this: web::Data<Self>, service: web::Data<Arc<Mutex<T>>>,
        param: web::Path<(String, String)>, req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        T: 'static + Fetching + Send + Sync,
        T::Item: rabbithole::entity::SingleEntity + Send + Sync,
    {
        if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
            return Ok(err_resp);
        }

        let uri = &this.uri().to_string();
        let path = req.uri().clone().into();

        match this.query.decode_path(&path) {
            Ok(query) => {
                let (id, related_field) = param.into_inner();
                match service
                    .lock()
                    .await
                    .fetch_relationship(&id, &related_field, uri, &path, &query)
                    .await
                {
                    Ok(OperationResultData { mut data, additional_links, additional_meta }) => {
                        data.extend_links(additional_links);
                        data.extend_meta(additional_meta);
                        Ok(new_json_api_resp(StatusCode::OK).json(data))
                    },
                    Err(err) => Ok(error_to_response(err)),
                }
            },
            Err(err) => Ok(error_to_response(err)),
        }
    }

    pub async fn fetch_related<T>(
        this: web::Data<Self>, service: web::Data<Arc<Mutex<T>>>,
        param: web::Path<(String, String)>, req: HttpRequest,
    ) -> Result<HttpResponse, actix_web::Error>
    where
        T: 'static + Fetching + Send + Sync,
        T::Item: rabbithole::entity::SingleEntity + Send + Sync,
    {
        if let Err(err_resp) = check_header(&this.jsonapi.version, &req.headers()) {
            return Ok(err_resp);
        }

        let uri = &this.uri().to_string();
        let path = req.uri().clone().into();

        match this.query.decode_path(&path) {
            Ok(query) => {
                let (id, related_field) = param.into_inner();
                match service
                    .lock()
                    .await
                    .fetch_related(&id, &related_field, uri, &path, &query)
                    .await
                {
                    Ok(item) => Ok(new_json_api_resp(StatusCode::OK).json(item)),
                    Err(err) => Ok(error_to_response(err)),
                }
            },
            Err(err) => Ok(error_to_response(err)),
        }
    }
}

// TODO: If this check should be put into the main logic rather than web-framework specific?
fn check_header(api_version: &JsonApiVersion, headers: &HeaderMap) -> Result<(), HttpResponse> {
    let content_type = headers.get(header::CONTENT_TYPE).map(|r| r.to_str().unwrap().to_string());
    let accept = headers.get(header::ACCEPT).map(|r| r.to_str().unwrap().to_string());
    RuleDispatcher::ContentTypeMustBeJsonApi(api_version, &content_type)
        .map_err(error_to_response)?;
    RuleDispatcher::AcceptHeaderShouldBeJsonApi(api_version, &accept).map_err(error_to_response)?;

    Ok(())
}

fn new_json_api_resp(status_code: StatusCode) -> HttpResponseBuilder {
    let mut resp = HttpResponse::build(status_code);
    resp.set_header(header::CONTENT_TYPE, JSON_API_HEADER);
    resp
}
