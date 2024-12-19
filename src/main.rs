use log::{debug, warn};
use ack_relay::{ReDBStore, Store, WebHook, WebHookInner};
use env_logger::Env;
use ntex_prometheus::PrometheusMiddleware;
use std::sync::Arc;

use ntex::web::{self, types::Json};

#[web::post("/")]
async fn new_ack(
    value: web::types::Json<WebHook>,
    db: web::types::State<Arc<ReDBStore>>,
) -> impl web::Responder {
    db.store(&value);
    web::HttpResponse::Ok().body("OK".to_string())
}

#[web::get("/")]
async fn get_current_ack(db: web::types::State<Arc<ReDBStore>>) -> impl web::Responder {
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
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let db_name = "db.redb";
    let port = 8080;
    let db = Arc::from(ReDBStore::open(db_name).expect("failed to open to store"));
    // Add basic cron
    let cron_db = db.clone();

    ntex::rt::spawn(async move {
        loop {
            let d2 = cron_db.clone();
            let keys = d2.get_entries();
            debug!("current keys {:?}", keys);
            let mut results = vec![];
            for (k, value) in keys {
                let result = handle_one_entry(k, value).await;
                match result {
                    Some(_) => {
                        debug!("Job is ok -> removing {}", &k);
                        results.push(k);
                    }
                    None => {
                        warn!("Job is ko -> will retry {}", &k);
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
            // will intercept the /metrics query and respond with the prometheus metrics
            .wrap(PrometheusMiddleware::new("/metrics"))
            .wrap(web::middleware::Logger::default())
            .service(get_current_ack)
            .service(new_ack)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
