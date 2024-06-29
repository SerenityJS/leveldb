// #![deny(clippy::all)]

mod compression;
mod leveldb;

pub mod serenityjs_leveldb {
  pub use crate::compression::ZlibCompressor;
  pub use crate::compression::RawZlibCompressor;
  pub use crate::leveldb::LevelDB;
}

// #[macro_use]
// extern crate napi_derive;

// #[napi]
// pub fn sum(a: i32, b: i32) -> i32 {
//   a + b
// }
