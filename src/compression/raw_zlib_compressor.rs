use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use rusty_leveldb::Compressor;

pub struct RawZlibCompressor {
  pub level: u8
}

impl RawZlibCompressor {
  pub fn new(level: u8) -> Self {
    assert!(level <= 10, "Invalid compression level");

    return Self {
      level,
    }
  }
}

impl Compressor for RawZlibCompressor {
  fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
    Ok(compress_to_vec(&block, self.level))
  }

  fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
     decompress_to_vec(&block).map_err(|e| rusty_leveldb::Status {
        code: rusty_leveldb::StatusCode::CompressionError,
        err: e.to_string(),
    })
  }
}