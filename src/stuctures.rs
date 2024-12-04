
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

use crate::{Method, WebHookInner};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WebHook {
    url: String,
    body: Option<Value>,
    method: Method,
}


impl WebHook {
    pub fn to_inner(&self) -> WebHookInner {
        // TODO: clean the clone ?
        WebHookInner {
            url: self.url.clone(),
            method: self.method.clone(),
            body: self
                .body
                .clone()
                .map(|e| serde_json::to_string(&e).unwrap()),
        }
    }
}

