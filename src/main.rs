use tokio_cron_scheduler::{Job, JobScheduler};

use std::sync::Arc;

use ack_relay::{Method, ReDBStore, Store, WebHook, WebHookInner};
use serde_json::Value;

// #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
// struct SomeKey {
//     foo: String,
//     bar: i32,
// }

use ntex::web::{self, types::Json};

#[web::post("/")]
async fn hello(
    value: web::types::Json<WebHook>,
    db: web::types::State<Arc<ReDBStore>>,
) -> impl web::Responder {
    db.store(&value);
    web::HttpResponse::Ok().body(format!("OK"))
}

#[web::get("/")]
async fn get_keys(db: web::types::State<Arc<ReDBStore>>) -> impl web::Responder {
    Json(db.get_entries())
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
                }
                None => {
                    // nothing to do
                }
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
    let db = Arc::from(ReDBStore::open(db_name).expect("failed to open to store"));
    // Add basic cron
    let cron_db = db.clone();
    sched
        .add(
            Job::new_async("1/10 * * * * *", move |_uuid, _l| {
                let d2 = cron_db.clone();
                Box::pin(async move {
                    println!("I run every 10 seconds");
                    let keys = d2.get_entries();
                    println!("current keys {:?}", keys);
                    let mut results = vec![];
                    for (k, value) in keys {
                        let result = handle_one_entry(k, value).await;
                        match result {
                            Some(_) => {
                                println!("Job is ok -> removing {}", &k);
                                results.push(k);
                            }
                            None => {
                                println!("Job is ko -> will retry {}", &k);
                            }
                        }
                    }
                    d2.validate_entries(results);
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
