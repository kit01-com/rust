use std::{borrow::Cow, io::ErrorKind, mem, str, time::Duration};

use compio::{
  BufResult,
  buf::{IntoInner, IoBuf},
  io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
  time::timeout,
};
use httparse::{EMPTY_HEADER, Error as HttparseError, Response, Status};
use itoa::Buffer;
use memchr::{memchr, memmem};

use crate::{
  conn::Conn,
  error::{Error, Result},
  target::Target,
};

const MAX_HEAD: usize = 64 * 1024;
const READ_STEP: usize = 8 * 1024;
const MAX_REDIRECT: usize = 8;
const MAX_LINE: usize = 1024;
const HEAD_STACK: usize = 64;
const HEAD_HEAP: usize = 256;
const REQ_TAIL: &[u8] =
  b"\r\nAccept-Encoding: identity\r\nUser-Agent: msrcget\r\nConnection: keep-alive\r\n\r\n";

/// 每个并发工作协程复用的请求、响应头与分块数据缓冲
pub(crate) struct WorkerBuf {
  pub(crate) req: Vec<u8>,
  pub(crate) head: Vec<u8>,
  pub(crate) data: Vec<u8>,
}

impl WorkerBuf {
  pub(crate) fn new(chunk_size: usize) -> Self {
    Self {
      req: Vec::with_capacity(256),
      head: Vec::with_capacity(1024),
      data: Vec::with_capacity(chunk_size),
    }
  }
}

#[derive(Clone, Copy)]
enum ContentRange {
  Range {
    start: u64,
    end: u64,
    total: Option<u64>,
  },
  Unsatisfiable {
    total: Option<u64>,
  },
}

#[derive(Clone, Copy)]
struct ParsedHead {
  status: u16,
  close: bool,
  chunked: bool,
  content_len: Option<u64>,
  content_range: Option<ContentRange>,
}

/// 探测资源总字节数，可能附带可复用的保活连接
pub(crate) async fn probe_size(
  target: &Target,
  conn: Option<Conn>,
  buf: &mut WorkerBuf,
  dur: Duration,
) -> Result<(u64, Option<Conn>)> {
  let (mut conn, body_off, meta, same_origin) = open_head(target, 0, 0, conn, buf, dur).await?;

  match meta.status {
    206 => {
      let Some(ContentRange::Range {
        start: 0,
        end: 0,
        total: Some(total),
      }) = meta.content_range
      else {
        return Err(Error::Probe);
      };
      if total == 0 {
        return Err(Error::Probe);
      }

      // 消耗掉探测请求的 1 字节响应体且与原源同域时才可归还保活池
      let keep =
        if same_origin && !meta.close && !meta.chunked && meta.content_len.unwrap_or(1) == 1 {
          let rem = buf.head.len() - body_off;
          if rem == 1 {
            Some(conn)
          } else if rem == 0 {
            match timeout(dur, conn.stream.read_exact([0u8; 1])).await {
              Ok(BufResult(Ok(()), _)) => Some(conn),
              _ => None,
            }
          } else {
            None
          }
        } else {
          None
        };
      Ok((total, keep))
    }
    416 => {
      if matches!(
        meta.content_range,
        Some(ContentRange::Unsatisfiable { total: Some(0) })
      ) {
        let keep = (same_origin
          && !meta.close
          && !meta.chunked
          && meta.content_len == Some(0)
          && buf.head.len() == body_off)
          .then_some(conn);
        Ok((0, keep))
      } else {
        Err(Error::Status(416))
      }
    }
    200 => {
      let Some(len) = meta.content_len else {
        return Err(Error::Probe);
      };
      let keep =
        (same_origin && len == 0 && !meta.close && !meta.chunked && buf.head.len() == body_off)
          .then_some(conn);
      Ok((len, keep))
    }
    code => Err(Error::Status(code)),
  }
}

/// 下载指定字节闭区间 `[start, start + expected_len - 1]` 到 `buf.data`，并按需返回可复用保活连接
pub(crate) async fn fetch_range(
  target: &Target,
  conn: Option<Conn>,
  buf: &mut WorkerBuf,
  start: u64,
  expected_len: usize,
  total_size: u64,
  dur: Duration,
) -> Result<Option<Conn>> {
  let end = start + expected_len as u64 - 1;
  let (mut conn, body_off, meta, same_origin) =
    open_head(target, start, end, conn, buf, dur).await?;

  match meta.status {
    206 => {
      let Some(ContentRange::Range {
        start: cr_start,
        end: cr_end,
        total,
      }) = meta.content_range
      else {
        return Err(Error::Proto);
      };
      if cr_start != start || cr_end != end {
        return Err(Error::Proto);
      }
      if let Some(t) = total
        && t != total_size
      {
        return Err(Error::Proto);
      }
      if !meta.chunked
        && let Some(cl) = meta.content_len
        && cl != expected_len as u64
      {
        return Err(Error::Proto);
      }
    }
    // 仅当单块覆盖全文件时允许服务端忽略 Range 返回 200
    200 if start == 0 && expected_len as u64 == total_size => {
      if !meta.chunked
        && let Some(cl) = meta.content_len
        && cl != expected_len as u64
      {
        return Err(Error::Proto);
      }
    }
    code => return Err(Error::Status(code)),
  }

  buf.data.clear();
  buf.data.reserve(expected_len);

  if meta.chunked {
    let keep = read_chunked(
      &mut conn,
      &mut buf.head,
      &mut buf.data,
      body_off,
      expected_len,
      meta.close,
      dur,
    )
    .await?;
    return Ok((same_origin && keep).then_some(conn));
  }

  let leftover = &buf.head[body_off..];
  let pre = leftover.len().min(expected_len);
  buf.data.extend_from_slice(&leftover[..pre]);
  let extra = leftover.len() > expected_len;

  if pre < expected_len {
    let data = mem::take(&mut buf.data);
    let BufResult(res, slice) = timeout(dur, conn.stream.read_exact(data.slice(pre..expected_len)))
      .await
      .map_err(|_| Error::Timeout)?;
    buf.data = slice.into_inner();
    res?;
  }

  if buf.data.len() != expected_len {
    return Err(Error::Proto);
  }

  let keep = same_origin && !meta.close && !extra;
  Ok(keep.then_some(conn))
}

#[inline]
fn is_redirect(status: u16) -> bool {
  matches!(status, 301 | 302 | 303 | 307 | 308)
}

async fn open_head(
  target: &Target,
  start: u64,
  end: u64,
  mut conn: Option<Conn>,
  buf: &mut WorkerBuf,
  dur: Duration,
) -> Result<(Conn, usize, ParsedHead, bool)> {
  let mut cur = Cow::Borrowed(target);

  for hop in 0..=MAX_REDIRECT {
    write_req(&mut buf.req, &cur, start, end);
    let (active_conn, body_off) = send_read_head(&cur, conn.take(), buf, dur).await?;
    let (meta, loc) = parse_head(&buf.head[..body_off])?;

    if is_redirect(meta.status) {
      if hop == MAX_REDIRECT {
        return Err(Error::Redirect);
      }
      let loc = loc.ok_or(Error::Proto)?;
      let next = cur.redirect(&loc)?;
      if !meta.close
        && !meta.chunked
        && meta.content_len == Some(0)
        && buf.head.len() == body_off
        && next.same_origin(&cur)
      {
        conn = Some(active_conn);
      }
      cur = Cow::Owned(next);
      continue;
    }

    let same_origin = matches!(cur, Cow::Borrowed(_)) || cur.same_origin(target);
    return Ok((active_conn, body_off, meta, same_origin));
  }

  Err(Error::Redirect)
}

async fn send_read_head(
  target: &Target,
  mut conn: Option<Conn>,
  buf: &mut WorkerBuf,
  dur: Duration,
) -> Result<(Conn, usize)> {
  let reused = conn.is_some();
  let mut active = match conn.take() {
    Some(c) => c,
    None => Conn::connect(target, dur).await?,
  };

  match send_and_read_once(&mut active, buf, dur).await {
    Ok(off) => Ok((active, off)),
    Err(err) if reused && is_stale_err(&err) => {
      // 复用连接可能在服务端空闲超时后已关闭，重建连接重试一次
      let mut fresh = Conn::connect(target, dur).await?;
      let off = send_and_read_once(&mut fresh, buf, dur).await?;
      Ok((fresh, off))
    }
    Err(err) => Err(err),
  }
}

async fn send_and_read_once(conn: &mut Conn, buf: &mut WorkerBuf, dur: Duration) -> Result<usize> {
  let req = mem::take(&mut buf.req);
  let BufResult(res, req_back) = timeout(dur, conn.stream.write_all(req))
    .await
    .map_err(|_| Error::Timeout)?;
  buf.req = req_back;
  res?;

  timeout(dur, conn.stream.flush())
    .await
    .map_err(|_| Error::Timeout)??;

  read_head(conn, &mut buf.head, dur).await
}

fn is_stale_err(err: &Error) -> bool {
  matches!(
    err,
    Error::Io(e)
      if matches!(
        e.kind(),
        ErrorKind::UnexpectedEof
          | ErrorKind::ConnectionReset
          | ErrorKind::BrokenPipe
          | ErrorKind::ConnectionAborted
      )
  )
}

fn write_req(req: &mut Vec<u8>, target: &Target, start: u64, end: u64) {
  req.clear();
  let mut s_buf = Buffer::new();
  let mut e_buf = Buffer::new();
  let s_str = s_buf.format(start);
  let e_str = e_buf.format(end);
  req.reserve(target.req_pre.len() + s_str.len() + 1 + e_str.len() + REQ_TAIL.len());
  req.extend_from_slice(&target.req_pre);
  req.extend_from_slice(s_str.as_bytes());
  req.push(b'-');
  req.extend_from_slice(e_str.as_bytes());
  req.extend_from_slice(REQ_TAIL);
}

async fn read_head(conn: &mut Conn, head: &mut Vec<u8>, dur: Duration) -> Result<usize> {
  head.clear();
  let mut scan = 0;

  loop {
    if let Some(i) = memmem::find(&head[scan..], b"\r\n\r\n") {
      return Ok(scan + i + 4);
    }
    if head.len() >= MAX_HEAD {
      return Err(Error::Proto);
    }
    // 末尾可能仅读到分隔符前缀，回退 3 字节保证跨包匹配
    scan = head.len().saturating_sub(3);
    let want = (MAX_HEAD - head.len()).min(READ_STEP);
    head.reserve(want);

    let next = mem::take(head);
    let BufResult(res, back) = timeout(dur, conn.stream.append(next))
      .await
      .map_err(|_| Error::Timeout)?;
    *head = back;

    let n = res?;
    if n == 0 {
      return Err(Error::Io(ErrorKind::UnexpectedEof.into()));
    }
  }
}

fn parse_head(raw: &[u8]) -> Result<(ParsedHead, Option<String>)> {
  let mut stack = [EMPTY_HEADER; HEAD_STACK];
  let mut resp = Response::new(&mut stack);
  match resp.parse(raw) {
    Ok(Status::Complete(_)) => extract_meta(&resp),
    Err(HttparseError::TooManyHeaders) => {
      let mut heap = vec![EMPTY_HEADER; HEAD_HEAP];
      let mut resp = Response::new(&mut heap);
      match resp.parse(raw) {
        Ok(Status::Complete(_)) => extract_meta(&resp),
        _ => Err(Error::Proto),
      }
    }
    _ => Err(Error::Proto),
  }
}

fn extract_meta(resp: &Response<'_, '_>) -> Result<(ParsedHead, Option<String>)> {
  let status = resp.code.ok_or(Error::Proto)?;
  let mut close = resp.version == Some(0);
  let mut seen_close = false;
  let mut chunked = false;
  let mut content_len: Option<u64> = None;
  let mut content_range: Option<ContentRange> = None;
  let mut location: Option<String> = None;

  for h in resp.headers.iter() {
    if h.name.eq_ignore_ascii_case("connection") {
      if let Ok(v) = str::from_utf8(h.value) {
        for tok in v.split(',') {
          let tok = tok.trim();
          if tok.eq_ignore_ascii_case("close") {
            close = true;
            seen_close = true;
          } else if tok.eq_ignore_ascii_case("keep-alive") && resp.version == Some(0) && !seen_close
          {
            close = false;
          }
        }
      }
    } else if h.name.eq_ignore_ascii_case("transfer-encoding") {
      if let Ok(v) = str::from_utf8(h.value)
        && v
          .split(',')
          .any(|tok| tok.trim().eq_ignore_ascii_case("chunked"))
      {
        chunked = true;
      }
    } else if h.name.eq_ignore_ascii_case("content-length") {
      let s = str::from_utf8(h.value).map_err(|_| Error::Proto)?.trim();
      let len = s.parse::<u64>().map_err(|_| Error::Proto)?;
      if let Some(prev) = content_len
        && prev != len
      {
        return Err(Error::Proto);
      }
      content_len = Some(len);
    } else if h.name.eq_ignore_ascii_case("content-range") {
      content_range = parse_content_range(h.value);
    } else if h.name.eq_ignore_ascii_case("location") && location.is_none() && is_redirect(status) {
      let s = str::from_utf8(h.value).map_err(|_| Error::Proto)?.trim();
      location = Some(s.to_owned());
    }
  }

  if chunked {
    content_len = None;
  }

  Ok((
    ParsedHead {
      status,
      close,
      chunked,
      content_len,
      content_range,
    },
    location,
  ))
}

fn parse_content_range(raw: &[u8]) -> Option<ContentRange> {
  let s = str::from_utf8(raw).ok()?.trim();
  let _ = s.get(..5).filter(|p| p.eq_ignore_ascii_case("bytes"))?;
  if !s.as_bytes().get(5).is_some_and(u8::is_ascii_whitespace) {
    return None;
  }
  let after = s.get(5..)?.trim_start();
  let (spec, total_str) = after.split_once('/')?;
  let spec = spec.trim();
  let total_str = total_str.trim();

  let total = if total_str == "*" {
    None
  } else {
    Some(total_str.parse::<u64>().ok()?)
  };

  if spec == "*" {
    return Some(ContentRange::Unsatisfiable { total });
  }

  let (start_s, end_s) = spec.split_once('-')?;
  let start = start_s.trim().parse::<u64>().ok()?;
  let end = end_s.trim().parse::<u64>().ok()?;
  if start > end {
    return None;
  }
  if let Some(t) = total
    && end >= t
  {
    return None;
  }

  Some(ContentRange::Range { start, end, total })
}

async fn read_chunked(
  conn: &mut Conn,
  head: &mut Vec<u8>,
  data: &mut Vec<u8>,
  body_off: usize,
  expected_len: usize,
  close: bool,
  dur: Duration,
) -> Result<bool> {
  let tail_len = head.len() - body_off;
  head.copy_within(body_off.., 0);
  head.truncate(tail_len);
  let mut pos = 0usize;

  loop {
    let line_end = read_line(conn, head, pos, dur).await?;
    let line = &head[pos..line_end];
    let size_part = memchr(b';', line).map_or(line, |i| &line[..i]);
    let size_str = str::from_utf8(size_part).map_err(|_| Error::Proto)?.trim();
    if size_str.is_empty() || !size_str.bytes().all(|b| b.is_ascii_hexdigit()) {
      return Err(Error::Proto);
    }
    let chunk_sz = usize::from_str_radix(size_str, 16).map_err(|_| Error::Proto)?;
    pos = line_end + 2;

    if chunk_sz == 0 {
      let mut trailer_len = 0usize;
      loop {
        let trailer_end = read_line(conn, head, pos, dur).await?;
        let line_len = trailer_end - pos;
        pos = trailer_end + 2;
        if line_len == 0 {
          break;
        }
        trailer_len += line_len + 2;
        if trailer_len > MAX_HEAD {
          return Err(Error::Proto);
        }
      }
      if data.len() != expected_len {
        return Err(Error::Proto);
      }
      return Ok(!close && pos == head.len());
    }

    if data
      .len()
      .checked_add(chunk_sz)
      .is_none_or(|n| n > expected_len)
    {
      return Err(Error::Proto);
    }

    let avail = head.len() - pos;
    let take = avail.min(chunk_sz);
    data.extend_from_slice(&head[pos..pos + take]);
    pos += take;

    let rem = chunk_sz - take;
    if rem > 0 {
      let cur = data.len();
      let owned = mem::take(data);
      let BufResult(res, slice) = timeout(dur, conn.stream.read_exact(owned.slice(cur..cur + rem)))
        .await
        .map_err(|_| Error::Timeout)?;
      *data = slice.into_inner();
      res?;
    }

    // 每个 chunk 数据体后固定跟 "\r\n"
    ensure_bytes(conn, head, &mut pos, 2, dur).await?;
    if &head[pos..pos + 2] != b"\r\n" {
      return Err(Error::Proto);
    }
    pos += 2;

    // 及时压缩缓冲避免长连接 chunked 解析积累头部残留
    if pos >= READ_STEP {
      let left = head.len() - pos;
      head.copy_within(pos.., 0);
      head.truncate(left);
      pos = 0;
    }
  }
}

async fn read_line(
  conn: &mut Conn,
  head: &mut Vec<u8>,
  pos: usize,
  dur: Duration,
) -> Result<usize> {
  let mut scan = pos;
  loop {
    if let Some(i) = memmem::find(&head[scan..], b"\r\n") {
      let end = scan + i;
      if end - pos > MAX_LINE {
        return Err(Error::Proto);
      }
      return Ok(end);
    }
    if head.len() - pos > MAX_LINE {
      return Err(Error::Proto);
    }
    scan = head.len().saturating_sub(1).max(pos);
    head.reserve(READ_STEP);
    let next = mem::take(head);
    let BufResult(res, back) = timeout(dur, conn.stream.append(next))
      .await
      .map_err(|_| Error::Timeout)?;
    *head = back;
    if res? == 0 {
      return Err(Error::Proto);
    }
  }
}

async fn ensure_bytes(
  conn: &mut Conn,
  head: &mut Vec<u8>,
  pos: &mut usize,
  need: usize,
  dur: Duration,
) -> Result<()> {
  if *pos > 0 && head.len() - *pos < need {
    let left = head.len() - *pos;
    head.copy_within(*pos.., 0);
    head.truncate(left);
    *pos = 0;
  }
  while head.len() - *pos < need {
    head.reserve(READ_STEP);
    let next = mem::take(head);
    let BufResult(res, back) = timeout(dur, conn.stream.append(next))
      .await
      .map_err(|_| Error::Timeout)?;
    *head = back;
    if res? == 0 {
      return Err(Error::Proto);
    }
  }
  Ok(())
}
