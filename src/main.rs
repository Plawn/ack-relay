use std::sync::Arc;

use ack_relay::{ReDBStore, Store, WebHook, WebHookInner};

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
    web::HttpResponse::Ok().body("OK".to_string())
}

#[web::get("/")]
async fn get_keys(db: web::types::State<Arc<ReDBStore>>) -> impl web::Responder {
    Json(db.get_entries())
}



async fn handle_one_entry(_key: u64, value: WebHookInner) -> Option<()> {
    // Can we reuse the same client ?
    let client = reqwest::Client::default();
    let req = {
        let m = value.method.for_req();
        let b = client.request(m, &value.url);
        match &value.get_body() {
            Some(body) => b.json(&body),
            None => b,
        }
    };
    let resp = req.send().await;
    resp.ok().filter(|e| e.status().as_u16() < 400).map(|_e| ())
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let db_name = "db.redb";
    let db = Arc::from(ReDBStore::open(db_name).expect("failed to open to store"));
    // Add basic cron
    let cron_db = db.clone();

    ntex::rt::spawn(async move {
        loop {
            let d2 = cron_db.clone();
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
            ntex::time::sleep(ntex::time::Seconds(10)).await;
        }
    });
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
