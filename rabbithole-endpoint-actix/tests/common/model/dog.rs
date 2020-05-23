extern crate rabbithole_derive as rbh_derive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(rbh_derive::EntityDecorator, Serialize, Deserialize, Clone, Debug)]
#[entity(type = "dogs")]
#[entity(service(crate::service::dog::DogService))]
#[entity(backend(actix))]
pub struct Dog {
    #[entity(id)]
    pub id: Uuid,
    pub name: String,
}

pub fn generate_dogs(len: usize) -> Vec<Dog> {
    let mut dogs = Vec::with_capacity(len);
    for i in 0 .. len {
        let uuid = uuid::Uuid::new_v4();
        dogs.push(Dog { id: uuid, name: i.to_string() });
    }
    dogs
}
