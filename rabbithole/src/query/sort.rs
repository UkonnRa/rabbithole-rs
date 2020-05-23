use crate::entity::SingleEntity;
use crate::model::error;
use crate::Result;
use std::cmp::Ordering;
use std::convert::TryFrom;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SortQuery(pub(crate) Vec<(String, OrderType)>);

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum OrderType {
    Asc,
    Desc,
}

impl TryFrom<Vec<(String, OrderType)>> for SortQuery {
    type Error = error::Error;

    fn try_from(map: Vec<(String, OrderType)>) -> Result<Self> {
        for (k, _) in &map {
            if k.contains('.') {
                return Err(error::Error::RelationshipPathNotSupported(&k, None));
            }
        }
        Ok(SortQuery(map))
    }
}

impl SortQuery {
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    pub fn insert_raw(&mut self, value: &str) -> Result<()> {
        for v in value.split(',').filter(|s| !s.is_empty()).map(ToString::to_string) {
            if v.starts_with('-') {
                self.insert((v.as_str()[1 ..]).into(), OrderType::Desc)?;
            } else {
                self.insert(v, OrderType::Asc)?;
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, key: String, value: OrderType) -> Result<()> {
        if key.contains('.') {
            return Err(error::Error::RelationshipPathNotSupported(&key, None));
        }
        self.0.push((key, value));
        Ok(())
    }

    pub fn sort<E: SingleEntity>(&self, entities: &mut [E]) {
        entities.sort_by(|a, b| Self::cmp_recur(a, b, &self.0))
    }

    fn cmp_recur<E: SingleEntity>(a: &E, b: &E, fields: &[(String, OrderType)]) -> Ordering {
        if let Some((field, order)) = fields.first() {
            let result = match order {
                OrderType::Asc => a.cmp_field(field, b),
                OrderType::Desc => b.cmp_field(field, a),
            }
            .unwrap_or(Ordering::Equal);
            if result == Ordering::Equal {
                SortQuery::cmp_recur(a, b, &fields[1 ..])
            } else {
                result
            }
        } else {
            Ordering::Equal
        }
    }
}
