use std::{
  io,
  sync::{Arc, OnceLock},
  time::Duration,
};

use compio::{
  BufResult,
  buf::{IoBuf, IoBufMut, IoVectoredBuf, IoVectoredBufMut},
  io::{AsyncRead, AsyncWrite},
  net::TcpStream,
  rustls::{ClientConfig, RootCertStore, crypto::ring::default_provider},
  time::timeout,
  tls::{TlsConnector, TlsStream},
};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{
  error::{Error, Result},
  target::Target,
};

/// 复用的全局 TLS 连接器，避免每次建连重复装载根证书
static TLS: OnceLock<TlsConnector> = OnceLock::new();

fn tls() -> Result<&'static TlsConnector> {
  if let Some(conn) = TLS.get() {
    return Ok(conn);
  }
  let mut root_store = RootCertStore::empty();
  root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
  let cfg = ClientConfig::builder_with_provider(Arc::new(default_provider()))
    .with_safe_default_protocol_versions()
    .map_err(|_| Error::Tls)?
    .with_root_certificates(root_store)
    .with_no_client_auth();
  let _ = TLS.set(TlsConnector::from(Arc::new(cfg)));
  TLS.get().ok_or(Error::Tls)
}

/// 零中间缓冲的 TCP / TLS 传输流
pub(crate) enum Stream {
  Plain(TcpStream),
  Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for Stream {
  #[inline]
  async fn read<B: IoBufMut>(&mut self, buf: B) -> BufResult<usize, B> {
    match self {
      Self::Plain(s) => s.read(buf).await,
      Self::Tls(s) => s.read(buf).await,
    }
  }

  #[inline]
  async fn read_vectored<V: IoVectoredBufMut>(&mut self, buf: V) -> BufResult<usize, V> {
    match self {
      Self::Plain(s) => s.read_vectored(buf).await,
      Self::Tls(s) => s.read_vectored(buf).await,
    }
  }
}

impl AsyncWrite for Stream {
  #[inline]
  async fn write<T: IoBuf>(&mut self, buf: T) -> BufResult<usize, T> {
    match self {
      Self::Plain(s) => s.write(buf).await,
      Self::Tls(s) => s.write(buf).await,
    }
  }

  #[inline]
  async fn write_vectored<T: IoVectoredBuf>(&mut self, buf: T) -> BufResult<usize, T> {
    match self {
      Self::Plain(s) => s.write_vectored(buf).await,
      Self::Tls(s) => s.write_vectored(buf).await,
    }
  }

  #[inline]
  async fn flush(&mut self) -> io::Result<()> {
    match self {
      Self::Plain(s) => s.flush().await,
      Self::Tls(s) => s.flush().await,
    }
  }

  #[inline]
  async fn shutdown(&mut self) -> io::Result<()> {
    match self {
      Self::Plain(s) => s.shutdown().await,
      Self::Tls(s) => s.shutdown().await,
    }
  }
}

/// 可按 Keep-Alive 复用的源连接
pub(crate) struct Conn {
  pub(crate) stream: Stream,
}

impl Conn {
  pub(crate) async fn connect(target: &Target, dur: Duration) -> Result<Self> {
    let tcp = timeout(dur, TcpStream::connect((target.host.as_str(), target.port)))
      .await
      .map_err(|_| Error::Timeout)??;
    let _ = tcp.set_nodelay(true);

    let stream = if target.https {
      let tls_stream = timeout(dur, tls()?.connect(&target.host, tcp))
        .await
        .map_err(|_| Error::Timeout)??;
      Stream::Tls(Box::new(tls_stream))
    } else {
      Stream::Plain(tcp)
    };

    Ok(Self { stream })
  }
}
