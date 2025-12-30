use std::{
  collections::HashSet,
  rc::Rc,
  sync::{Mutex, Once},
};

use miniz_oxide::deflate::CompressionLevel;
use napi::{bindgen_prelude::Buffer, Error, Result, Status::GenericFailure};
use napi_derive::napi;
use once_cell::sync::Lazy;
use rusty_leveldb::{compressor::NoneCompressor, CompressorList, DB};

use crate::serenityjs_leveldb::{RawZlibCompressor, ZlibCompressor};

const COMPRESSION_LEVEL: u8 = CompressionLevel::DefaultLevel as u8;

/// Tracks opened DB paths inside the *same Node process* (shared across worker threads).
/// This prevents accidental concurrent opens of the same path, which can lead to crashes
/// depending on the underlying DB implementation/locking behavior.
static OPEN_PATHS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Ensures we install the panic hook only once.
static PANIC_HOOK: Once = Once::new();

fn init_panic_hook() {
  PANIC_HOOK.call_once(|| {
    std::panic::set_hook(Box::new(|info| {
      eprintln!("\n[Rust panic] serenityjs-leveldb: {info}");

      // Backtrace is controlled via RUST_BACKTRACE=1 / full
      // This line helps on newer Rust versions; harmless if backtrace is disabled.
      eprintln!("{:?}", std::backtrace::Backtrace::capture());
    }));
  });
}

#[napi]
pub struct Leveldb {
  /// Wrapped DB handle. `None` after close (prevents double-close and use-after-close).
  rusty_leveldb: Option<DB>,

  /// Stored for global open-path guard cleanup.
  path: String,
}

#[napi]
impl Leveldb {
  #[napi(ts_return_type = "Leveldb")]
  /**
   * Open a database
   * @param path The path to the database
   */
  pub fn open(path: String) -> Result<Self> {
    init_panic_hook();

    // Prevent multiple opens of the same path within the same process (including worker threads).
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

    let mut opt = rusty_leveldb::Options::default();

    // Mojang create a custom compressor list:
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/include/leveldb/options.h#L123
    //
    // Sample config:
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/mcpe_sample_setup.cpp#L24-L28
    //
    // Compression IDs:
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/include/leveldb/zlib_compressor.h#L38
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/include/leveldb/zlib_compressor.h#L48
    //
    // Used in format:
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/table/format.cc#L125-L150
    let mut list = CompressorList::new();
    list.set_with_id(0, NoneCompressor::default());
    list.set_with_id(2, ZlibCompressor::new(COMPRESSION_LEVEL));
    list.set_with_id(4, RawZlibCompressor::new(COMPRESSION_LEVEL));

    // NOTE: rusty_leveldb expects an Rc here.
    // If the underlying library accesses this from background threads, Rc can be problematic.
    // The OPEN_PATHS guard also helps avoid concurrent access patterns that tend to trigger crashes.
    opt.compressor_list = Rc::new(list);

    // Set compressor
    // Minecraft bedrock may use other id than 4 however default is 4:
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/table/table_builder.cc#L152
    //
    // There is a bug in this library that you have to open a database with the same compression type as it was written to.
    // Mojang can fall back to raw if compressed is larger:
    // https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/table/table_builder.cc#L155-L165
    opt.compressor = 4;

    let db = match DB::open(path.clone(), opt) {
      Ok(db) => db,
      Err(e) => {
        // Roll back open-path reservation.
        if let Ok(mut set) = OPEN_PATHS.lock() {
          set.remove(&path);
        }
        return Err(Error::new(GenericFailure, e.to_string()));
      }
    };

    Ok(Self {
      rusty_leveldb: Some(db),
      path,
    })
  }

  fn db_mut(&mut self) -> Result<&mut DB> {
    self
      .rusty_leveldb
      .as_mut()
      .ok_or_else(|| Error::new(GenericFailure, "Database is closed".to_string()))
  }

  fn release_path_guard(&self) {
    if let Ok(mut set) = OPEN_PATHS.lock() {
      set.remove(&self.path);
    }
  }

  #[napi]
  /**
   * Close the database
   */
  pub fn close(&mut self) -> Result<()> {
    // Take the DB out so we cannot accidentally double-close.
    let Some(mut db) = self.rusty_leveldb.take() else {
      // Already closed (idempotent).
      self.release_path_guard();
      return Ok(());
    };

    let res = db
      .close()
      .map_err(|e| Error::new(GenericFailure, e.to_string()));

    // Always release the guard so the process can reopen if needed.
    self.release_path_guard();

    res
  }

  #[napi]
  /**
   * Flushes the database, ensuring all writes are persisted to disk.
   */
  pub fn flush(&mut self) -> Result<()> {
    self
      .db_mut()?
      .flush()
      .map_err(|e| Error::new(GenericFailure, e.to_string()))
  }

  #[napi]
  /**
   * Get a value from the database
   * @param key The key to get the value for
   * @returns The value for the key
   */
  pub fn get(&mut self, key: Buffer) -> Result<Buffer> {
    let bytes = key.as_ref();

    let value = match self.db_mut()?.get(bytes) {
      Some(value) => value,
      None => return Err(Error::new(GenericFailure, "Key not found".to_string())),
    };

    // Keep this conservative (works whether `value` is Vec<u8> or a slice-like type).
    Ok(Buffer::from(value.to_vec()))
  }

  #[napi]
  /**
   * Put a value into the database
   * @param key The key to put the value under
   * @param value The value to put into the database
   */
  pub fn put(&mut self, key: Buffer, value: Buffer) -> Result<()> {
    let key_bytes = key.as_ref();
    let value_bytes = value.as_ref();

    self
      .db_mut()?
      .put(key_bytes, value_bytes)
      .map_err(|e| Error::new(GenericFailure, e.to_string()))
  }

  #[napi]
  /**
   * Delete a key from the database
   * @param key The key to delete
   */
  pub fn delete(&mut self, key: Buffer) -> Result<()> {
    let bytes = key.as_ref();

    self
      .db_mut()?
      .delete(bytes)
      .map_err(|e| Error::new(GenericFailure, e.to_string()))
  }
}

impl Drop for Leveldb {
  fn drop(&mut self) {
    // Ensure we don't leave the process guard stuck if JS drops without calling close().
    if let Some(mut db) = self.rusty_leveldb.take() {
      let _ = db.close();
    }
    self.release_path_guard();
  }
}
