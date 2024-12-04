use std::sync::Arc;

use ack_relay::{ReDBStore, Store, WebHook, WebHookInner};

use ntex::web::{self, types::Json};
use prometheus::Registry;

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

#[web::get("/metrics")]
pub async fn metrics(
    registry: web::types::State<Arc<Registry>>,
    encoder: web::types::State<Arc<prometheus::TextEncoder>>,
) -> impl web::Responder {
    let metrics = registry.gather();
    encoder.encode_to_string(&metrics).unwrap()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let db_name = "db.redb";
    let db = Arc::from(ReDBStore::open(db_name).expect("failed to open to store"));
    // Add basic cron
    let cron_db = db.clone();
    let encoder = Arc::from(prometheus::TextEncoder::new());
    let prom_service = ack_relay::prom::prepare_prom();
    let registry = Arc::from(prom_service.registry.clone());
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
            .state(encoder.clone())
            .state(registry.clone())
            .wrap(prom_service.clone())
            .service(get_keys)
            .service(hello)
            .service(metrics)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
