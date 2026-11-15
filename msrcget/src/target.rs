use itoa::Buffer;
use url::{ParseError, Url};

use crate::error::{Error, Result};

const REQ_GET: &[u8] = b"GET ";
const REQ_MID: &[u8] = b" HTTP/1.1\r\nHost: ";
const REQ_RANGE: &[u8] = b"\r\nRange: bytes=";

/// 已解析的 HTTP/HTTPS 下载源地址
#[derive(Clone, Debug)]
pub(crate) struct Target {
  pub(crate) url: Url,
  pub(crate) https: bool,
  pub(crate) host: String,
  pub(crate) port: u16,
  /// 预拼接的请求头固定前缀
  pub(crate) req_pre: Box<[u8]>,
}

impl Target {
  pub(crate) fn parse(raw: &str) -> Result<Self> {
    Self::from_url(Url::parse(raw)?)
  }

  pub(crate) fn redirect(&self, loc: &str) -> Result<Self> {
    Self::from_url(self.url.join(loc)?)
  }

  pub(crate) fn same_origin(&self, other: &Self) -> bool {
    self.https == other.https && self.port == other.port && self.host == other.host
  }

  fn from_url(url: Url) -> Result<Self> {
    let https = match url.scheme() {
      "http" => false,
      "https" => true,
      other => return Err(Error::Scheme(other.to_owned())),
    };

    let raw_host = url.host_str().ok_or(ParseError::EmptyHost)?;
    let port = url.port_or_known_default().ok_or(ParseError::InvalidPort)?;
    // TCP 连接与 SNI 不接受 IPv6 方括号，Host 头需保留方括号
    let host = raw_host
      .strip_prefix('[')
      .and_then(|s| s.strip_suffix(']'))
      .unwrap_or(raw_host)
      .to_owned();

    let def_port = if https { 443 } else { 80 };
    let mut port_buf = Buffer::new();
    let port_str = (port != def_port).then(|| port_buf.format(port));

    let path = url.path();
    let query = url.query();
    let cap = REQ_GET.len()
      + path.len()
      + query.map_or(0, |q| 1 + q.len())
      + REQ_MID.len()
      + raw_host.len()
      + port_str.map_or(0, |p| 1 + p.len())
      + REQ_RANGE.len();

    let mut req_pre = Vec::with_capacity(cap);
    req_pre.extend_from_slice(REQ_GET);
    req_pre.extend_from_slice(path.as_bytes());
    if let Some(q) = query {
      req_pre.push(b'?');
      req_pre.extend_from_slice(q.as_bytes());
    }
    req_pre.extend_from_slice(REQ_MID);
    req_pre.extend_from_slice(raw_host.as_bytes());
    if let Some(p) = port_str {
      req_pre.push(b':');
      req_pre.extend_from_slice(p.as_bytes());
    }
    req_pre.extend_from_slice(REQ_RANGE);

    Ok(Self {
      url,
      https,
      host,
      port,
      req_pre: req_pre.into_boxed_slice(),
    })
  }
}
