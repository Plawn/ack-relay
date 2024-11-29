use std::{hash::DefaultHasher, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use std::{fmt::Debug, sync::Arc};

use ack_relay::Bincode;
use redb::{Database, Error, Range, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::{ Hash, Hasher};

// #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
// struct SomeKey {
//     foo: String,
//     bar: i32,
// }

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Hash)]
enum Method {
    GET,
    POST,
    PATCH,
    PUT,
    DELETE,
}

impl Method {
    fn for_reqwest(&self) -> reqwest::Method {
        match self {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PATCH => reqwest::Method::PATCH,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct WebHook {
    url: String,
    body: Option<Value>,
    method: Method,
}

impl WebHook {
    fn to_inner(&self) -> WebHookInner {
        return WebHookInner {
            url: self.url.clone(),
            method: self.method.clone(),
            body: self.body.clone().map(|e| serde_json::to_string(&e).unwrap()),
        };
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Hash)]
struct WebHookInner {
    url: String,
    body: Option<String>,
    method: Method,
}

// #[derive(Debug, Serialize, Deserialize, PartialEq)]
// struct Entry {
//     value: WebHook,
// }

// const TABLE: TableDefinition<Bincode<SomeKey>, Bincode<SomeValue>> =
//     TableDefinition::new("my_data");

const TABLE: TableDefinition<u64, Bincode<WebHookInner>> = TableDefinition::new("my_data");

fn open_db(filename: &str) -> Result<Database, Error> {
    if std::fs::exists(filename)? {
        let d = Database::open(filename)?;
        Ok(d)
    } else {
        let d = Database::create(filename)?;
        Ok(d)
    }
}

use ntex::web::{self, types::Json};

#[web::post("/")]
async fn hello(
    value: web::types::Json<WebHook>,
    db: web::types::State<Arc<Database>>,
) -> impl web::Responder {
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(TABLE).expect("failed to open table");
        let v = value.into_inner().to_inner();
        let mut hasher = DefaultHasher::new();
        v.hash(&mut hasher);
        let hash = hasher.finish();
        table.insert(hash, v).expect("failed to insert");
    }
    write_txn.commit().expect("failed to commit");

    // web::HttpResponse::Ok().body(format!("Hello world! {}", value))
    web::HttpResponse::Ok().body(format!("OK"))
}

fn get_current_entries(db: &Database) -> Vec<(u64, WebHookInner)> {
    let read_txn = db.begin_read().unwrap();
    let table = read_txn.open_table(TABLE);
    match table {
        Ok(t) => {
            let res = t
                .iter()
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| (e.0, e.1))
                .map(|e| (e.0.value(), e.1.value()))
                .collect::<Vec<_>>();
            res
        }
        Err(_) => vec![],
    }
}

#[web::get("/")]
async fn get_keys(db: web::types::State<Arc<Database>>) -> impl web::Responder {
    let d = db.as_ref();
    Json(get_current_entries(d))
}

use reqwest;

async fn handle_one_entry(key: u64, value: WebHookInner) -> Option<()> {
    let client = reqwest::Client::new();
    println!("hanlding {:?}", &value);
    match value.method {
        Method::GET => client
            .get(value.url)
            .send()
            .await
            .map(|e| e.error_for_status().ok())
            .ok()
            .flatten()
            .map(|e| ()),
        Method::POST | Method::PATCH | Method::PUT | Method::DELETE => {
            let mut b = client.request(value.method.for_reqwest(), value.url);
            match value.body {
                Some(c) => {
                    let parsed: Value = serde_json::from_str(&c).unwrap();
                    b = b.json(&parsed)
                },
                None => {
                    // nothing to do
                },
            }
            b.send()
                .await
                .map(|e| e.error_for_status().ok())
                .ok()
                .flatten()
                .map(|e| ())
        }
    }
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let db_name = "db.redb";
    let sched = JobScheduler::new()
        .await
        .expect("failed to create scheduler");
    let db = Arc::from(open_db(db_name).expect("failed to open db"));
    // Add basic cron
    let cron_db = db.clone();
    sched
        .add(
            Job::new_async("1/10 * * * * *", move |_uuid, _l| {
                let d2 = cron_db.clone();
                Box::pin(async move {
                    println!("I run every 10 seconds");
                    let keys = get_current_entries(d2.as_ref());
                    println!("current keys {:?}", keys);
                    let mut write_txn = d2.begin_write().expect("failed to get write tx");
                    {
                        let mut table = write_txn.open_table(TABLE).expect("failed to open table");
                        for (k, value) in keys {
                            let result = handle_one_entry(k, value).await;
                            match result {
                                Some(_) => {
                                    println!("Job is ok -> removing {}", &k);
                                    table.remove(k).expect("failed to remove key");
                                }
                                None => {
                                    println!("Job is ko -> will retry {}", &k);
                                }
                            }
                        }
                    }
                    write_txn.commit().expect("failed to commit");
                })
            })
            .unwrap(),
        )
        .await
        .expect("failed to start scheduler");
    sched.start().await.unwrap();
    web::HttpServer::new(move || {
        web::App::new()
            .state(db.clone())
            .service(get_keys)
            .service(hello)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
