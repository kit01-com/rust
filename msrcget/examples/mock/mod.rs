use std::{mem::take, net::SocketAddr, sync::Arc, time::Duration};

use compio::{
  BufResult,
  io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
  net::{TcpListener, TcpStream},
  runtime::spawn,
  time::sleep,
};

/// 启动本地多源演示服务，支持按块注入损坏与人为响应延迟
pub async fn start_mirror(
  data: Arc<[u8]>,
  chunk_size: usize,
  corrupt_chunk: Option<usize>,
  delay: Duration,
) -> String {
  let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
  let addr: SocketAddr = listener.local_addr().expect("local_addr");
  spawn(async move {
    loop {
      let Ok((stream, _)) = listener.accept().await else {
        break;
      };
      let d = Arc::clone(&data);
      spawn(serve_conn(stream, d, chunk_size, corrupt_chunk, delay)).detach();
    }
  })
  .detach();
  format!("http://{addr}/demo.bin")
}

async fn serve_conn(
  mut stream: TcpStream,
  data: Arc<[u8]>,
  chunk_size: usize,
  corrupt_chunk: Option<usize>,
  delay: Duration,
) {
  let mut buf = Vec::with_capacity(1024);
  loop {
    let end = loop {
      if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        break pos + 4;
      }
      let next = take(&mut buf);
      let BufResult(res, back) = stream.append(next).await;
      buf = back;
      if res.unwrap_or(0) == 0 {
        return;
      }
    };
    let req = String::from_utf8_lossy(&buf[..end]).into_owned();
    buf.drain(..end);

    let (s, e) = req
      .lines()
      .find_map(|l| {
        let (k, v) = l.split_once(':')?;
        if !k.trim().eq_ignore_ascii_case("range") {
          return None;
        }
        let (a, b) = v.trim().strip_prefix("bytes=")?.split_once('-')?;
        Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
      })
      .unwrap_or((0, data.len().saturating_sub(1)));

    if data.is_empty() {
      return;
    }

    if !delay.is_zero() && !(s == 0 && e == 0) {
      sleep(delay).await;
    }

    let mut body = data[s..=e].to_vec();
    if Some(s / chunk_size) == corrupt_chunk && !(s == 0 && e == 0) {
      body[0] ^= 0xff;
    }

    let head = format!(
      "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {s}-{e}/{}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
      data.len(),
      body.len()
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(&body);
    let BufResult(res, _) = stream.write_all(out).await;
    if res.is_err() || stream.flush().await.is_err() {
      return;
    }
  }
}
