use napi_derive::napi;

#[napi]
pub struct LevelDB {
  pub path: String,
}

#[napi]
impl LevelDB {
  #[napi(constructor)]
  pub fn new(path: String) -> Self {
    Self {
      path,
    }
  }
}