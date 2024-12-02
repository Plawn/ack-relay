use crate::{Bincode, Store, WebHook, WebHookInner};
use std::hash::DefaultHasher;

use redb::{Database, Error, ReadableTable, TableDefinition};
use std::hash::{Hash, Hasher};

pub struct ReDBStore {
    db: Database,
}

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

impl ReDBStore {
    pub fn open(filename: &str) -> Result<Self, Error> {
        open_db(filename).map(|db| Self {
            db
        })
    }
}

impl Store for ReDBStore {
    fn store(&self, value: &WebHook) {
        let write_txn = self.db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(TABLE).expect("failed to open table");
            let v = value.to_inner();
            let mut hasher = DefaultHasher::new();
            v.hash(&mut hasher);
            let hash = hasher.finish();
            table.insert(hash, v).expect("failed to insert");
        }
        write_txn.commit().expect("failed to commit");
    }

    fn get_entries(&self) -> Vec<(u64, WebHookInner)> {
        let read_txn = self.db.begin_read().unwrap();
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
    fn validate_entries(&self, entries: Vec<u64>) {
        let write_txn = self.db.begin_write().expect("failed to get write tx");
        {
            let mut table = write_txn.open_table(TABLE).expect("failed to open table");
            for k in entries {
                table.remove(k).expect("failed to remove key");
            }
        }
        write_txn.commit().expect("failed to commit");
    }
}
