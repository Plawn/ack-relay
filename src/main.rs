use std::fmt::Debug;

use ack_relay::Bincode;
use redb::{Database, Error, Range, TableDefinition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct SomeKey {
    foo: String,
    bar: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SomeValue {
    foo: [f64; 3],
    bar: bool,
}

const TABLE: TableDefinition<Bincode<SomeKey>, Bincode<SomeValue>> =
    TableDefinition::new("my_data");

fn open_db(filename: &str) -> Result<Database, Error> {
    if std::fs::exists(filename)? {
        let d = Database::open(filename)?;
        Ok(d)
    } else {
        let d = Database::create(filename)?;
        Ok(d)
    }
}

use ntex::web;

#[web::get("/")]
async fn hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hello world!")
}

// fn main() -> Result<(), Error> {
//     let some_key = SomeKey {
//         foo: "hello world".to_string(),
//         bar: 42,
//     };
//     let some_value = SomeValue {
//         foo: [1., 2., 3.],
//         bar: true,
//     };
//     let lower = SomeKey {
//         foo: "a".to_string(),
//         bar: 42,
//     };
//     let upper = SomeKey {
//         foo: "z".to_string(),
//         bar: 42,
//     };

//     let db = open_db("bincode_keys.redb")?;
//     let write_txn = db.begin_write()?;
//     {
//         let mut table = write_txn.open_table(TABLE)?;

//         table.insert(&some_key, &some_value).unwrap();
//     }
//     write_txn.commit()?;

//     let read_txn = db.begin_read()?;
//     let table = read_txn.open_table(TABLE)?;

//     let mut iter: Range<Bincode<SomeKey>, Bincode<SomeValue>> = table.range(lower..upper).unwrap();
//     assert_eq!(iter.next().unwrap().unwrap().1.value(), some_value);
//     assert!(iter.next().is_none());

//     Ok(())
// }
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let mut sched = JobScheduler::new().await.unwrap();

    // Add basic cron job
    sched.add(
        Job::new("1/10 * * * * *", |_uuid, _l| {
            println!("I run every 10 seconds");
        }).unwrap()
    ).await.unwrap();
    sched.start().await.unwrap();
    web::HttpServer::new(|| {
        web::App::new()
            .service(hello)
            // .service(echo)
            // .route("/hey", web::get().to(manual_hello))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}