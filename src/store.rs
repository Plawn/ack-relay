
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::fmt::Debug;

use crate::WebHook;

pub trait Store {
    fn store(&self, value: &WebHook);
    fn get_entries(&self) -> Vec<(u64, WebHookInner)>;
    fn validate_entries(&self, values: Vec<u64>);
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Hash)]
pub struct WebHookInner {
    pub url: String,
    pub body: Option<String>,
    pub method: Method,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Hash)]
pub enum Method {
    GET,
    POST,
    PATCH,
    PUT,
    DELETE,
}

impl Method {
    pub fn for_reqwest(&self) -> reqwest::Method {
        match self {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PATCH => reqwest::Method::PATCH,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
        }
    }
}
