pub mod filter;
pub mod page;
pub mod sort;

use crate::model::error;

use crate::Result;

use crate::query::filter::FilterQuery;
use crate::query::page::PageQuery;
use crate::query::sort::{OrderType, SortQuery};

use crate::entity::SingleEntity;
use crate::model::link::{Link, Links, RawUri};
use itertools::Itertools;
use percent_encoding::percent_decode_str;
use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
use regex::Regex;

use std::collections::{HashMap, HashSet};

pub type IncludeQuery = HashSet<String>;
pub type FieldsQuery = HashMap<String, HashSet<String>>;

#[derive(Debug, Deserialize, Clone)]
pub struct FilterSettings {
    #[serde(rename = "type")]
    pub ty: String,
}

impl Default for FilterSettings {
    fn default() -> Self { Self { ty: "Rsql".to_string() } }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PageSettings {
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuerySettings {
    #[serde(default = "default_size")]
    pub default_size: usize,
    #[serde(default)]
    pub raw_encode: bool,
    #[serde(default)]
    pub filter: FilterSettings,
    #[serde(default)]
    pub page: Option<PageSettings>,
}
fn default_size() -> usize { 10 }

impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            default_size: default_size(),
            raw_encode: Default::default(),
            filter: Default::default(),
            page: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Query {
    pub settings: QuerySettings,
    /// When include is:
    ///   1. `None`: all included fields will be added
    ///   2. `Some(<empty-query>)`: no included field will be added
    ///   3. `Some(<some-query>)`:
    ///     1. No query matches the existing fields: like branch 2
    ///     2. Otherwise: all matched fields will be added
    pub include: Option<IncludeQuery>,
    /// If `ty` is marked in `fields` map:
    ///   1. If `fields[<ty>]` is not empty: all resources of this type will remove the `attributes` and `relationships` which are not in `fields[<ty>]`
    ///   2. If `fields[<ty>]` is empty: all the fields of the resources with this type will be removed
    /// Else: retain all fields
    pub fields: FieldsQuery,
    /// When sort is:
    ///   1. empty: no sorting at all, clients should not expect the order of the result
    ///   2. some values, but none of the values matches: like branch 1
    ///   3. some values, and some of the items matches: sorting result with the order of the matched sort-query item
    pub sort: SortQuery,
    pub page: Option<PageQuery>,
    pub filter: FilterQuery,
}

impl Query {
    pub fn query<E: SingleEntity>(
        &self, mut data: Vec<E>, uri: &str, path: &RawUri,
    ) -> Result<(Vec<E>, Links)> {
        self.sort.sort(&mut data);
        let data = self.filter.filter(data)?;
        let (data, relat_pages) = if let Some(page) = &self.page {
            page.page(&data)?
        } else {
            (data.as_slice(), Default::default())
        };
        let relat_pages: Result<Links> = relat_pages
            .into_iter()
            .map(|(k, v)| {
                let relat_query = Query { page: Some(v), ..self.clone() };
                self.settings
                    .encode_path(path, &relat_query)
                    .map(|path| Link::new(uri, path))
                    .map(|e| (k, e))
            })
            .collect();
        Ok((data.to_vec(), relat_pages?))
    }
}

impl ToString for Query {
    fn to_string(&self) -> String {
        let include_query = match &self.include {
            None => Default::default(),
            Some(data) if data.is_empty() => Default::default(),
            Some(data) => format!("include={}", data.iter().join(",")),
        };
        let fields_query: Vec<String> = self
            .fields
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| format!("fields[{}]={}", k, v.iter().join(",")))
            .collect();
        let sort_query = self
            .sort
            .0
            .iter()
            .map(|(k, ty)| {
                let ty_str = match ty {
                    OrderType::Asc => "",
                    OrderType::Desc => "-",
                };
                format!("{}{}", ty_str, k)
            })
            .join(",");
        let page_query = self.page.as_ref().map(ToString::to_string).unwrap_or_default();
        let filter_query = self.filter.to_string();
        let mut vec = vec![include_query, page_query, filter_query];
        if !sort_query.is_empty() {
            vec.push(format!("sort={}", sort_query));
        }
        vec.extend(fields_query);
        vec.into_iter().filter(|s| !s.is_empty()).join("&")
    }
}

lazy_static! {
    static ref KEY_REGEX: Regex = Regex::new(r#"(?P<name>\w+)\[(?P<param>[\w\-_@]+)\]"#).unwrap();
    static ref CHAR_SET: AsciiSet = NON_ALPHANUMERIC.remove(b'=').remove(b'&');
}

impl QuerySettings {
    pub fn encode_path(&self, path: &RawUri, query: &Query) -> Result<RawUri> {
        let RawUri(path) = path;
        let query_str = query.to_string();
        let query_str = if self.raw_encode {
            query_str
        } else {
            percent_encode(query_str.as_bytes(), &CHAR_SET).to_string()
        };
        let path_and_query = format!("{}?{}", path.path(), query_str);
        let builder = http::Uri::builder();
        let builder = builder.path_and_query(path_and_query.as_bytes());
        builder.build().map(RawUri).map_err(|err| error::Error::InvalidUri(&err, None))
    }

    pub fn decode_path(&self, path: &RawUri) -> Result<Query> {
        let RawUri(path) = path;
        let mut include_query: IncludeQuery = Default::default();
        let mut include_query_exist = false;
        let mut sort_query: SortQuery = Default::default();
        let mut filter_map: HashMap<String, String> = Default::default();
        let mut fields_map: FieldsQuery = Default::default();
        let mut page_map: HashMap<String, String> = Default::default();

        if let Some(query_str) = path.query() {
            let query_str = percent_decode_str(query_str)
                .decode_utf8()
                .map_err(|err| error::Error::NotUtf8String(query_str, &err, None))?;

            for (key, value) in query_str.split('&').filter_map(|s| {
                let kv_pair: Vec<&str> = s.splitn(2, '=').collect();
                if kv_pair.len() == 2 && !kv_pair[0].is_empty() {
                    Some((kv_pair[0], kv_pair[1]))
                } else {
                    None
                }
            }) {
                if key == "include" {
                    include_query_exist = true;

                    for v in value.split(',').filter(|s| !s.is_empty()).map(ToString::to_string) {
                        include_query.insert(v);
                    }
                    continue;
                }

                if key == "sort" {
                    sort_query.insert_raw(value)?;
                    continue;
                }

                if let Some(cap) = (&KEY_REGEX as &Regex).captures(key.as_ref()) {
                    if let (Some(name), Some(param)) = (cap.name("name"), cap.name("param")) {
                        let name = name.as_str();
                        let param = param.as_str();

                        if name == "fields" {
                            let values: HashSet<String> = value
                                .split(',')
                                .filter(|s| !s.is_empty())
                                .map(ToString::to_string)
                                .collect();
                            if let Some(origin_fields) = fields_map.get_mut(param) {
                                for v in values {
                                    origin_fields.insert(v);
                                }
                            } else {
                                fields_map.insert(param.into(), values);
                            }
                        } else if name == "filter" && !value.is_empty() {
                            filter_map.insert(param.into(), value.to_string());
                        } else if name == "page" {
                            page_map.insert(param.into(), value.to_string());
                        }
                    }
                }
            }
        }
        let include = if include_query_exist { Some(include_query) } else { None };
        let sort = sort_query;
        let page = if let Some(_page_settings) = self.page.as_ref() {
            Some(PageQuery::new(&self, &page_map)?)
        } else {
            None
        };
        let filter = FilterQuery::new(&self.filter, &filter_map)?;
        let query =
            Query { settings: self.clone(), include, fields: fields_map, sort, page, filter };

        Ok(query)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::link::RawUri;
    use crate::query::{PageSettings, QuerySettings, CHAR_SET};
    use percent_encoding::percent_encode;

    #[test]
    fn to_string_test() {
        let query = percent_encode(
            b"filter[book]=publishDate>1454638927411,genre=out=('\
                              Literary Fiction','Science \
                              Fiction')&include=authors&fields[book]=title,authors&\
                              fields[author]=name&page[offset]=3&page[limit]=2",
            &CHAR_SET,
        )
        .to_string();
        let uri: RawUri = format!("/author/1?{}", query).parse().unwrap();
        let settings = QuerySettings {
            default_size: 10,
            page: Some(PageSettings { ty: "OffsetBased".into() }),
            ..Default::default()
        };

        match settings.decode_path(&uri) {
            Ok(query_data) => assert_eq!(
                query.split('&').count(),
                settings
                    .encode_path(&uri, &query_data)
                    .unwrap()
                    .query()
                    .unwrap()
                    .split('&')
                    .count()
            ),
            Err(err) => unreachable!("error: {}", err),
        }
    }
}
