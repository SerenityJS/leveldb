use std::sync::mpsc;

use napi::{Error, Result, Status::GenericFailure};

use super::types::{DbCommand, DbResult};

pub async fn request<T: Send + 'static>(
  tx: mpsc::Sender<DbCommand>,
  build: impl FnOnce(mpsc::Sender<DbResult<T>>) -> DbCommand + Send + 'static,
) -> Result<T> {
  tokio::task::spawn_blocking(move || {
    let (resp_tx, resp_rx) = mpsc::channel();

    tx.send(build(resp_tx))
      .map_err(|_| Error::new(GenericFailure, "DB worker is not running".to_string()))?;

    match resp_rx.recv() {
      Ok(Ok(value)) => Ok(value),
      Ok(Err(err)) => Err(Error::new(GenericFailure, err)),
      Err(_) => Err(Error::new(
        GenericFailure,
        "DB worker dropped response channel".to_string(),
      )),
    }
  })
  .await
  .map_err(|e| Error::new(GenericFailure, format!("Join error: {e}")))?
}