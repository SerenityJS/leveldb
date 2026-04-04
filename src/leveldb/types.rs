use std::{
  sync::{mpsc, Mutex},
  thread::JoinHandle,
};

pub type DbResult<T> = std::result::Result<T, String>;

pub enum DbCommand {
  Get {
    key: Vec<u8>,
    resp: mpsc::Sender<DbResult<Option<Vec<u8>>>>,
  },
  Put {
    key: Vec<u8>,
    value: Vec<u8>,
    resp: mpsc::Sender<DbResult<()>>,
  },
  Delete {
    key: Vec<u8>,
    resp: mpsc::Sender<DbResult<()>>,
  },
  GetWorkerThreadId {
    resp: mpsc::Sender<DbResult<String>>,
  },
  Close {
    resp: mpsc::Sender<DbResult<()>>,
  },
}

pub struct WorkerState {
  pub tx: mpsc::Sender<DbCommand>,
  pub join: Mutex<Option<JoinHandle<()>>>,
}
