use std::{rc::Rc, sync::Arc};

/// 分块校验接口，由调用方实现并传入下载器
pub trait Verify {
  /// 校验第 `idx` 块（0 基序号）的数据，返回 `true` 表示校验通过
  fn verify(&self, idx: usize, data: &[u8]) -> bool;
}

impl Verify for () {
  #[inline]
  fn verify(&self, _idx: usize, _data: &[u8]) -> bool {
    true
  }
}

impl<F> Verify for F
where
  F: Fn(usize, &[u8]) -> bool,
{
  #[inline]
  fn verify(&self, idx: usize, data: &[u8]) -> bool {
    self(idx, data)
  }
}

impl<T: Verify + ?Sized> Verify for Rc<T> {
  #[inline]
  fn verify(&self, idx: usize, data: &[u8]) -> bool {
    (**self).verify(idx, data)
  }
}

impl<T: Verify + ?Sized> Verify for Arc<T> {
  #[inline]
  fn verify(&self, idx: usize, data: &[u8]) -> bool {
    (**self).verify(idx, data)
  }
}
