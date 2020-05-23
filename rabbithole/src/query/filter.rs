use crate::model::error;

use crate::Result;

use rsql::Expr;

use rsql::parser::rsql::RsqlParser;
use rsql::parser::Parser;
use rsql::Comparison;
use rsql::Constraint;
use rsql::Operator;

use crate::entity::SingleEntity;
use crate::query::FilterSettings;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::HashMap;

pub trait FilterData: Sized + ToString {
    fn new(params: &HashMap<String, String>) -> Result<Self>;

    fn filter<E: SingleEntity>(&self, entities: Vec<E>) -> Result<Vec<E>>;
}

/// Example:
/// `?include=authors&filter[book]=title==*Foo*&filter[author]=name!='Orson Scott Card'`
/// where key is self type or relationship name
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Default, Clone)]
pub struct RsqlFilterData(HashMap<String, Expr>);

impl ToString for RsqlFilterData {
    fn to_string(&self) -> String {
        self.0.iter().map(|(k, v)| format!("filter[{}]={}", k, v.to_string())).join("&")
    }
}

impl FilterData for RsqlFilterData {
    fn new(params: &HashMap<String, String>) -> Result<Self> {
        let mut res: HashMap<String, Expr> = Default::default();
        for (k, v) in params.iter() {
            if k.contains('.') {
                return Err(error::Error::RelationshipPathNotSupported(&k, None));
            }
            let expr = RsqlParser::default()
                .parse_to_node(v)
                .map_err(|_| error::Error::UnmatchedFilterItem("Rsql", &k, &v, None))?;
            res.insert(k.clone(), expr);
        }
        Ok(RsqlFilterData(res))
    }

    fn filter<E: SingleEntity>(&self, mut entities: Vec<E>) -> Result<Vec<E>> {
        for (ty_or_relat, expr) in &self.0 {
            entities = entities
                .into_iter()
                .filter_map(|r| {
                    match (&E::ty() == ty_or_relat, Self::filter_on_attributes(expr, &r)) {
                        (true, Ok(true)) => Some(Ok(r)),
                        (true, Ok(false)) => None,
                        (true, Err(err)) => Some(Err(err)),
                        (false, _) => {
                            Some(Err(error::Error::RsqlFilterOnRelatedNotImplemented(None)))
                        },
                    }
                })
                .collect::<Result<Vec<E>>>()?;
        }
        Ok(entities)
    }
}

impl RsqlFilterData {
    pub fn filter_on_attributes<E: SingleEntity>(expr: &Expr, entity: &E) -> Result<bool> {
        let ent: bool = match &expr {
            Expr::Item(Constraint { selector, comparison, arguments }) => {
                if let Ok(field) = entity.attributes().get_field(&selector) {
                    if comparison == &Comparison::EQUAL() && arguments.0.len() == 1 {
                        let arg: &str = arguments.0.first().unwrap();
                        field.eq_with_str(arg, &selector)?
                    } else if comparison == &Comparison::NOT_EQUAL() && arguments.0.len() == 1 {
                        let arg: &str = arguments.0.first().unwrap();
                        !(field.eq_with_str(arg, &selector)?)
                    } else if comparison == &Comparison::GREATER_THAN() && arguments.0.len() == 1 {
                        let arg: &str = arguments.0.first().unwrap();
                        field.cmp_with_str(arg, &selector)? == Ordering::Greater
                    } else if comparison == &Comparison::GREATER_THAN_OR_EQUAL()
                        && arguments.0.len() == 1
                    {
                        let arg: &str = arguments.0.first().unwrap();
                        let res = field.cmp_with_str(arg, &selector)?;
                        res == Ordering::Greater || res == Ordering::Equal
                    } else if comparison == &Comparison::LESS_THAN() && arguments.0.len() == 1 {
                        let arg: &str = arguments.0.first().unwrap();
                        let res = field.cmp_with_str(arg, &selector)?;
                        res == Ordering::Less
                    } else if comparison == &Comparison::LESS_THAN_OR_EQUAL()
                        && arguments.0.len() == 1
                    {
                        let arg: &str = arguments.0.first().unwrap();
                        let res = field.cmp_with_str(arg, &selector)?;
                        res == Ordering::Less || res == Ordering::Equal
                    } else if comparison == &Comparison::IN() {
                        arguments.0.iter().any(|s| field.eq_with_str(s, &selector).is_ok())
                    } else if comparison == &Comparison::OUT() {
                        arguments
                            .0
                            .iter()
                            .find(|s| field.eq_with_str(s, &selector).is_ok())
                            .is_none()
                    } else {
                        return Err(error::Error::UnsupportedRsqlComparison(
                            &comparison.get_symbols(),
                            arguments.0.len(),
                            None,
                        ));
                    }
                } else {
                    return Err(error::Error::FieldNotExist(&selector, None));
                }
            },
            Expr::Node(op, left, right) => {
                let left = Self::filter_on_attributes(left, entity)?;
                match op {
                    Operator::And => left && Self::filter_on_attributes(right, entity)?,
                    Operator::Or => left || Self::filter_on_attributes(right, entity)?,
                }
            },
        };
        Ok(ent)
    }
}

#[derive(Debug, Clone)]
pub enum FilterQuery {
    Rsql(RsqlFilterData),
}

impl ToString for FilterQuery {
    fn to_string(&self) -> String {
        match &self {
            FilterQuery::Rsql(data) => data.to_string(),
        }
    }
}

impl Default for FilterQuery {
    fn default() -> Self { Self::Rsql(Default::default()) }
}

impl FilterQuery {
    pub fn new(settings: &FilterSettings, params: &HashMap<String, String>) -> Result<FilterQuery> {
        if &settings.ty == "Rsql" {
            RsqlFilterData::new(params).map(FilterQuery::Rsql)
        } else {
            Err(error::Error::InvalidFilterType(&settings.ty, None))
        }
    }

    pub fn filter<E: SingleEntity>(&self, entities: Vec<E>) -> Result<Vec<E>> {
        match &self {
            FilterQuery::Rsql(map) => RsqlFilterData::filter(map, entities),
        }
    }
}
