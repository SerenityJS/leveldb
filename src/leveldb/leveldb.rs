use std::rc::Rc;

use miniz_oxide::deflate::CompressionLevel;
use napi::{bindgen_prelude::Buffer, Error, Result, Status::GenericFailure};
use napi_derive::napi;
use rusty_leveldb::{compressor::NoneCompressor, CompressorList, DB};

use crate::serenityjs_leveldb::{RawZlibCompressor, ZlibCompressor};

#[napi]
pub struct Leveldb {
  rusty_leveldb: DB,
}

const COMPRESSION_LEVEL: u8 = CompressionLevel::DefaultLevel as u8;

#[napi]
impl Leveldb {
  #[napi(ts_return_type="Leveldb")]
  /**
   * Open a database
   * @param path The path to the database
  */
  pub fn open(path: String) -> Result<Self> {
    let mut opt = rusty_leveldb::Options::default();

    // Mojang create a custom [compressor list](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/include/leveldb/options.h#L123)
    // Sample config for compressor list can be find in [here](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/mcpe_sample_setup.cpp#L24-L28)
    // 
    // Their compression id can be find in [here](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/include/leveldb/zlib_compressor.h#L38)
    // and [here](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/include/leveldb/zlib_compressor.h#L48)
    // 
    // Compression id will be use in [here](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/table/format.cc#L125-L150)
    let mut list = CompressorList::new();
    list.set_with_id(0, NoneCompressor::default());
    list.set_with_id(2, ZlibCompressor::new(COMPRESSION_LEVEL));
    list.set_with_id(4, RawZlibCompressor::new(COMPRESSION_LEVEL));
    opt.compressor_list = Rc::new(list);

    // Set compressor
    // Minecraft bedrock may use other id than 4 however default is 4. [Mojang's implementation](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/table/table_builder.cc#L152)
    //
    // There is a bug in this library that you have to open a database with the same compression type as it was written to.
    // If raw data is smaller than compression, Mojang will use raw data. [Mojang's implementation](https://github.com/reedacartwright/rbedrock/blob/fb32a899da4e15c1aaa0d6de2b459e914e183516/src/leveldb-mcpe/table/table_builder.cc#L155-L165)
    // There is a small chance that compression id 0 exists, you should use compression id 0 to write it.
    opt.compressor = 4;

    let db = match DB::open(path, opt) {
      Ok(db) => db,
      Err(e) => return Err(Error::new(GenericFailure, e.to_string()))
    };

    return Ok(Self {
      rusty_leveldb: db
    })
  }

  #[napi]
  /**
   * Close the database
  */
  pub fn close(&mut self) -> Result<()> {
    // Close the database
    // Catch any errors and return them as a js error
    match self.rusty_leveldb.close() {
      Ok(_) => Ok(()),
      Err(e) => return Err(Error::new(GenericFailure, e.to_string()))
    }
  }

  #[napi]
  /**
   * Get a value from the database
   * @param key The key to get the value for
   * @returns The value for the key
  */
  pub fn get(&mut self, key: Buffer) -> Result<Buffer> {
    // Turn the key into a byte array reference
    let bytes = key.as_ref();

    // Get the value from the database
    let value = match self.rusty_leveldb.get(bytes) {
      Some(value) => value,
      None => return Err(Error::new(GenericFailure, "Key not found".to_string()))
    };

    // Return the value as a js buffer
    return Ok(Buffer::from(value))
  }

  #[napi]
  /**
   * Put a value into the database
   * @param key The key to put the value under
   * @param value The value to put into the database
  */
  pub fn put(&mut self, key: Buffer, value: Buffer) -> Result<()> {
    // Turn the key and value into byte array references
    let key_bytes = key.as_ref();
    let value_bytes = value.as_ref();

    // Put the key and value into the database
    // Catch any errors and return them as a js error
    match self.rusty_leveldb.put(key_bytes, value_bytes) {
      Ok(_) => (),
      Err(e) => return Err(Error::new(GenericFailure, e.to_string()))
    }

    // Return nothing if successful
    return Ok(())
  }

  #[napi]
  /**
   * Delete a key from the database
   * @param key The key to delete
  */
  pub fn delete(&mut self, key: Buffer) -> Result<()> {
    // Turn the key into a byte array reference
    let bytes = key.as_ref();

    // Delete the key from the database
    // Catch any errors and return them as a js error
    match self.rusty_leveldb.delete(bytes) {
      Ok(_) => (),
      Err(e) => return Err(Error::new(GenericFailure, e.to_string()))
    }

    // Return nothing if successful
    return Ok(())
  }
}
