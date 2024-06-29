use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib};
use rusty_leveldb::Compressor;

pub struct ZlibCompressor {
  pub level: u8
}

impl ZlibCompressor {
  pub fn new(level: u8) -> Self {
    assert!(level <= 10, "Invalid compression level");

    return Self {
      level,
    }
  }
}

impl Compressor for ZlibCompressor {
  fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
    Ok(compress_to_vec_zlib(&block, self.level))
  }

  fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
    decompress_to_vec_zlib(&block).map_err(|e| rusty_leveldb::Status {
        code: rusty_leveldb::StatusCode::CompressionError,
        err: e.to_string(),
    })
  }
}