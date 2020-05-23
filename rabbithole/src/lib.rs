#![feature(associated_type_defaults)]
#![feature(step_trait)]
#![feature(step_trait_ext)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

use crate::model::error::Error;

pub type Result<T> = std::result::Result<T, Error>;
pub const JSON_API_HEADER: &str = "application/vnd.api+json";

pub mod entity;
pub mod model;
pub mod operation;
pub mod query;
pub mod rule;
