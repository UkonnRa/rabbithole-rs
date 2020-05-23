extern crate rabbithole_derive as rbh_derive;
use crate::model::dog::Dog;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(rbh_derive::EntityDecorator, Serialize, Deserialize, Clone, Debug)]
#[entity(type = "people")]
#[entity(service(crate::service::human::HumanService))]
#[entity(backend(actix))]
pub struct Human {
    #[entity(id)]
    pub id: Uuid,
    pub name: String,
    #[entity(to_many)]
    pub dogs: Vec<Dog>,
}

impl Human {
    pub fn add_dogs(&mut self, dogs: &mut Vec<Dog>) {
        self.dogs.append(dogs);
        self.dogs.dedup_by_key(|dog| dog.id);
    }

    pub fn remove_dogs(&mut self, dog_ids: &[String]) {
        let new_dogs: Vec<Dog> = self
            .dogs
            .iter()
            .filter(|dog| !dog_ids.contains(&dog.id.to_string()))
            .cloned()
            .collect();
        self.dogs = new_dogs;
    }
}

impl From<&[Dog]> for Human {
    fn from(dogs: &[Dog]) -> Self {
        let uuid = uuid::Uuid::new_v4();
        Self { id: uuid, name: uuid.to_string(), dogs: dogs.to_vec() }
    }
}
