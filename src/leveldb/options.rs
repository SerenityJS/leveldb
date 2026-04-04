use std::rc::Rc;

use miniz_oxide::deflate::CompressionLevel;
use rusty_leveldb::{compressor::NoneCompressor, CompressorList};

use crate::serenityjs_leveldb::{RawZlibCompressor, ZlibCompressor};

const COMPRESSION_LEVEL: u8 = CompressionLevel::DefaultLevel as u8;

pub fn create_options() -> rusty_leveldb::Options {
  let mut opt = rusty_leveldb::Options::default();

  let mut list = CompressorList::new();
  list.set_with_id(0, NoneCompressor::default());
  list.set_with_id(2, ZlibCompressor::new(COMPRESSION_LEVEL));
  list.set_with_id(4, RawZlibCompressor::new(COMPRESSION_LEVEL));

  opt.compressor_list = Rc::new(list);
  opt.compressor = 4;
  opt
}