use std::{sync::mpsc, thread};

use rusty_leveldb::DB;

use super::{
  options::create_options,
  types::{DbCommand, DbResult},
};

pub fn run_db_worker(path: String, rx: mpsc::Receiver<DbCommand>) {
  let mut db = match DB::open(path, create_options()) {
    Ok(db) => db,
    Err(err) => {
      eprintln!("Failed to open DB in worker: {err}");
      return;
    }
  };

  while let Ok(cmd) = rx.recv() {
    match cmd {
      DbCommand::Get { key, resp } => {
        let result: DbResult<Option<Vec<u8>>> = Ok(db.get(&key).map(|v| v.to_vec()));
        let _ = resp.send(result);
      }

      DbCommand::Put { key, value, resp } => {
        let result = db.put(&key, &value).map_err(|e| e.to_string());
        let _ = resp.send(result);
      }

      DbCommand::Delete { key, resp } => {
        let result = db.delete(&key).map_err(|e| e.to_string());
        let _ = resp.send(result);
      }

      DbCommand::GetWorkerThreadId { resp } => {
        let id = format!("{:?}", thread::current().id());
        let _ = resp.send(Ok(id));
      }

      DbCommand::Close { resp } => {
        let result = db.close().map_err(|e| e.to_string());
        let _ = resp.send(result);
        break;
      }
    }
  }
}
