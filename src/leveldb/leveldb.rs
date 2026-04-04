use std::{
  collections::HashSet,
  sync::{mpsc, Arc, Mutex, Once},
  thread,
};

use napi::{bindgen_prelude::Buffer, Error, Result, Status::GenericFailure};
use napi_derive::napi;
use once_cell::sync::Lazy;
use rusty_leveldb::DB;

use super::{
  options::create_options,
  requests::request,
  types::{DbCommand, WorkerState},
  worker::run_db_worker,
};

static OPEN_PATHS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static PANIC_HOOK: Once = Once::new();

fn init_panic_hook() {
  PANIC_HOOK.call_once(|| {
    std::panic::set_hook(Box::new(|info| {
      eprintln!("\n[Rust panic] serenityjs-leveldb: {info}");
      eprintln!("{:?}", std::backtrace::Backtrace::capture());
    }));
  });
}

#[napi]
pub struct Leveldb {
  worker: Arc<WorkerState>,
  path: String,
}

#[napi]
impl Leveldb {
  #[napi(ts_return_type = "Leveldb")]
  pub fn open(path: String) -> Result<Self> {
    init_panic_hook();

    {
      let mut set = OPEN_PATHS
        .lock()
        .map_err(|_| Error::new(GenericFailure, "OPEN_PATHS lock poisoned".to_string()))?;

      if set.contains(&path) {
        return Err(Error::new(
          GenericFailure,
          format!("Database already open in this process: {path}"),
        ));
      }

      set.insert(path.clone());
    }

    {
      let mut test_db = DB::open(path.clone(), create_options()).map_err(|e| {
        if let Ok(mut set) = OPEN_PATHS.lock() {
          set.remove(&path);
        }
        Error::new(GenericFailure, e.to_string())
      })?;

      test_db.close().map_err(|e| {
        if let Ok(mut set) = OPEN_PATHS.lock() {
          set.remove(&path);
        }
        Error::new(GenericFailure, e.to_string())
      })?;
    }

    let (tx, rx) = mpsc::channel();
    let thread_path = path.clone();

    let join = thread::spawn(move || {
      run_db_worker(thread_path, rx);
    });

    Ok(Self {
      worker: Arc::new(WorkerState {
        tx,
        join: Mutex::new(Some(join)),
      }),
      path,
    })
  }

  fn release_path_guard(&self) {
    if let Ok(mut set) = OPEN_PATHS.lock() {
      set.remove(&self.path);
    }
  }

  #[napi]
  pub async fn get(&self, key: Buffer) -> Result<Option<Buffer>> {
    let tx = self.worker.tx.clone();
    let key = key.to_vec();

    let value = request(tx, move |resp| DbCommand::Get { key, resp }).await?;
    Ok(value.map(Buffer::from))
  }

  #[napi]
  pub async fn put(&self, key: Buffer, value: Buffer) -> Result<()> {
    let tx = self.worker.tx.clone();
    let key = key.to_vec();
    let value = value.to_vec();

    request(tx, move |resp| DbCommand::Put { key, value, resp }).await
  }

  #[napi]
  pub async fn get_worker_thread_id(&self) -> Result<String> {
    let tx = self.worker.tx.clone();
    request(tx, move |resp| DbCommand::GetWorkerThreadId { resp }).await
  }

  #[napi]
  pub async fn close(&self) -> Result<()> {
    let tx = self.worker.tx.clone();

    request(tx, move |resp| DbCommand::Close { resp }).await?;

    if let Ok(mut join) = self.worker.join.lock() {
      if let Some(handle) = join.take() {
        let _ = handle.join();
      }
    }

    self.release_path_guard();
    Ok(())
  }
}

impl Drop for Leveldb {
  fn drop(&mut self) {
    if let Ok(mut join) = self.worker.join.lock() {
      if join.is_some() {
        let (resp_tx, _resp_rx) = mpsc::channel();
        let _ = self.worker.tx.send(DbCommand::Close { resp: resp_tx });

        if let Some(handle) = join.take() {
          let _ = handle.join();
        }
      }
    }

    self.release_path_guard();
  }
}
