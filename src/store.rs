use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

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

impl WebHookInner  {
    pub fn get_body(&self) -> Option<serde_json::Value> {
        return self.body.as_ref().map(|e| serde_json::from_str(e).ok()).flatten();
    }
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Hash)]
pub enum Method {
    GET,
    POST,
    PATCH,
    PUT,
    DELETE,
}

// impl From<Method> for ntex::http::Method {
//     fn from(value: Method) -> Self {
//         match value {
//             Method::GET => ntex::http::Method::GET,
//             Method::POST => ntex::http::Method::POST,
//             Method::PATCH => ntex::http::Method::PATCH,
//             Method::PUT => ntex::http::Method::PUT,
//             Method::DELETE => ntex::http::Method::DELETE,
//         }
//     }
// }

impl Method {
    pub fn for_req(&self) -> reqwest::Method {
        match self {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PATCH => reqwest::Method::PATCH,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
        }
    }
}

impl From<&Method> for reqwest::Method {
    fn from(value: &Method) -> Self {
        match value {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PATCH => reqwest::Method::PATCH,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
        }
    }
}

// impl From<Method> for reqwest::Method {
//     fn from(value: Method) -> Self {
//         match value {
//             Method::GET => reqwest::Method::GET,
//             Method::POST => reqwest::Method::POST,
//             Method::PATCH => reqwest::Method::PATCH,
//             Method::PUT => reqwest::Method::PUT,
//             Method::DELETE => reqwest::Method::DELETE,
//         }
//     }
// }

impl From<Method> for &str {
    fn from(value: Method) -> Self {
        match value {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PATCH => "PATCH",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
        }
    }
}

impl From<&Method> for &str {
    fn from(value: &Method) -> Self {
        match value {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PATCH => "PATCH",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
        }
    }
}
