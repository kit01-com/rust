use std::{io, result};

use thiserror::Error;
use url::ParseError;

/// 下载过程中可能产生的错误
#[derive(Error, Debug)]
pub enum Error {
  /// 下载源列表为空
  #[error("empty source list")]
  EmptySrc,
  /// 分块大小非法（例如为 0）
  #[error("invalid chunk size")]
  ChunkSize,
  /// URL 解析失败
  #[error(transparent)]
  Url(#[from] ParseError),
  /// 不支持的协议方案
  #[error("unsupported scheme: {0}")]
  Scheme(String),
  /// 底层 I/O 错误
  #[error(transparent)]
  Io(#[from] io::Error),
  /// TLS 配置初始化失败
  #[error("tls config failed")]
  Tls,
  /// 单次请求或读取超时
  #[error("request timeout")]
  Timeout,
  /// 重定向次数超过上限
  #[error("too many redirects")]
  Redirect,
  /// 服务端返回非预期 HTTP 状态码
  #[error("http status {0}")]
  Status(u16),
  /// HTTP 响应格式或头部非法
  #[error("invalid http response")]
  Proto,
  /// 无法从服务端探测文件总大小
  #[error("unable to probe content length")]
  Probe,
  /// 临时续传文件已被其他进程锁定
  #[error("part file locked by another process")]
  Locked,
  /// 某分块已遍历全部下载源但仍未通过下载与校验
  #[error("chunk {idx} exhausted all sources")]
  Exhausted {
    /// 失败的分块序号（0 基）
    idx: usize,
  },
}

/// 库统一返回类型
pub type Result<T> = result::Result<T, Error>;
