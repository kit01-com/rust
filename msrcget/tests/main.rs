use std::{
  cell::RefCell,
  fs::{File as StdFile, metadata, read},
  io::Write,
  mem::take,
  net::SocketAddr,
  rc::Rc,
  sync::Arc,
  time::{Duration, Instant},
};

use aok::{OK, Void};
use compio::{
  BufResult,
  io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
  net::{TcpListener, TcpStream},
  runtime::spawn,
  time::sleep,
};
use crc32fast::Hasher;
use log_init::init;
use msrcget::{Error, Get, MAX_CHUNK, MIN_CHUNK, Verify, get, part_path, plan_chunk_size};
use static_init::constructor;
use tempfile::tempdir;

#[constructor(0)]
extern "C" fn _log_init() {
  init();
}

#[derive(Clone, Copy)]
enum Fault {
  Ok,
  Corrupt,
  Status500,
  Chunked,
}

#[derive(Clone)]
struct MirrorOpt {
  data: Arc<[u8]>,
  chunk_size: usize,
  delay: Duration,
  fault_li: Rc<RefCell<Vec<Fault>>>,
  redirect_to: Option<String>,
  expect_path: Option<String>,
  empty_416: bool,
  req_li: Rc<RefCell<Vec<(u64, u64)>>>,
}

impl MirrorOpt {
  fn healthy(data: Arc<[u8]>, chunk_size: usize) -> Self {
    Self {
      data,
      chunk_size,
      delay: Duration::ZERO,
      fault_li: Rc::new(RefCell::new(Vec::new())),
      redirect_to: None,
      expect_path: None,
      empty_416: false,
      req_li: Rc::new(RefCell::new(Vec::new())),
    }
  }
}

async fn start_mirror(opt: MirrorOpt) -> (String, SocketAddr) {
  let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
  let addr = listener.local_addr().expect("local_addr");
  let url = format!("http://{addr}/file.bin");

  spawn(async move {
    loop {
      let Ok((stream, _)) = listener.accept().await else {
        break;
      };
      let opt_conn = opt.clone();
      spawn(async move {
        serve_conn(stream, opt_conn).await;
      })
      .detach();
    }
  })
  .detach();

  (url, addr)
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
  buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn parse_range_header(head: &str, total: usize) -> Option<(usize, usize)> {
  for line in head.lines() {
    if let Some((k, v)) = line.split_once(':')
      && k.trim().eq_ignore_ascii_case("range")
    {
      let spec = v.trim().strip_prefix("bytes=")?;
      let (s_str, e_str) = spec.split_once('-')?;
      let s: usize = s_str.trim().parse().ok()?;
      let e: usize = if e_str.trim().is_empty() {
        total.saturating_sub(1)
      } else {
        e_str.trim().parse().ok()?
      };
      return Some((s, e));
    }
  }
  None
}

async fn write_slice(stream: &mut TcpStream, raw: Vec<u8>) -> bool {
  let BufResult(res, _) = stream.write_all(raw).await;
  if res.is_err() {
    return false;
  }
  stream.flush().await.is_ok()
}

async fn serve_conn(mut stream: TcpStream, opt: MirrorOpt) {
  let mut buf = Vec::with_capacity(1024);

  loop {
    let head_end = loop {
      if let Some(pos) = find_header_end(&buf) {
        break pos;
      }
      if buf.len() > 8192 {
        return;
      }
      let next = take(&mut buf);
      let BufResult(res, back) = stream.append(next).await;
      buf = back;
      match res {
        Ok(0) | Err(_) => return,
        Ok(_) => {}
      }
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    buf.drain(..head_end);

    if let Some(expect) = &opt.expect_path {
      let req_path = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("");
      if req_path != expect {
        let resp =
          b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n".to_vec();
        if !write_slice(&mut stream, resp).await {
          return;
        }
        continue;
      }
    }

    if let Some(loc) = &opt.redirect_to {
      let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {loc}\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n"
      );
      if !write_slice(&mut stream, resp.into_bytes()).await {
        return;
      }
      continue;
    }

    let total = opt.data.len();
    let Some((start, end)) = parse_range_header(&head, total) else {
      let resp =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: keep-alive\r\n\r\n");
      let mut out = resp.into_bytes();
      out.extend_from_slice(&opt.data);
      if !write_slice(&mut stream, out).await {
        return;
      }
      continue;
    };

    if total == 0 {
      let resp = if opt.empty_416 {
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */0\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n".to_owned()
      } else {
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n".to_owned()
      };
      if !write_slice(&mut stream, resp.into_bytes()).await {
        return;
      }
      continue;
    }

    if start >= total || end >= total || start > end {
      let resp = format!(
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n"
      );
      if !write_slice(&mut stream, resp.into_bytes()).await {
        return;
      }
      continue;
    }

    if start == 0 && end == 0 {
      let mut out = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-0/{total}\r\nContent-Length: 1\r\nConnection: keep-alive\r\n\r\n"
      )
      .into_bytes();
      out.push(opt.data[0]);
      if !write_slice(&mut stream, out).await {
        return;
      }
      continue;
    }

    opt.req_li.borrow_mut().push((start as u64, end as u64));

    if !opt.delay.is_zero() {
      sleep(opt.delay).await;
    }

    let chunk_idx = start / opt.chunk_size.max(1);
    let fault = {
      let li = opt.fault_li.borrow();
      li.get(chunk_idx).copied().unwrap_or(Fault::Ok)
    };

    match fault {
      Fault::Status500 => {
        let resp =
          b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n"
            .to_vec();
        if !write_slice(&mut stream, resp).await {
          return;
        }
      }
      Fault::Corrupt => {
        let mut body = opt.data[start..=end].to_vec();
        body[0] ^= 0xff;
        let len = body.len();
        let mut out = format!(
          "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {len}\r\nConnection: keep-alive\r\n\r\n"
        )
        .into_bytes();
        out.extend_from_slice(&body);
        if !write_slice(&mut stream, out).await {
          return;
        }
      }
      Fault::Chunked => {
        let body = &opt.data[start..=end];
        let mut out = format!(
          "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\n\r\n"
        )
        .into_bytes();
        let mid = body.len() / 2;
        for part in [&body[..mid], &body[mid..]] {
          if !part.is_empty() {
            out.extend_from_slice(format!("{:x}\r\n", part.len()).as_bytes());
            out.extend_from_slice(part);
            out.extend_from_slice(b"\r\n");
          }
        }
        out.extend_from_slice(b"0\r\n\r\n");
        if !write_slice(&mut stream, out).await {
          return;
        }
      }
      Fault::Ok => {
        let body = &opt.data[start..=end];
        let len = body.len();
        let mut out = format!(
          "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {len}\r\nConnection: keep-alive\r\n\r\n"
        )
        .into_bytes();
        out.extend_from_slice(body);
        if !write_slice(&mut stream, out).await {
          return;
        }
      }
    }
  }
}

fn gen_payload(len: usize) -> Arc<[u8]> {
  let mut buf = Vec::with_capacity(len);
  let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
  for _ in 0..len {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    buf.push((state >> 24) as u8);
  }
  buf.into()
}

struct CrcVerify {
  crc_li: Vec<u32>,
}

impl CrcVerify {
  fn from_data(data: &[u8], chunk_size: usize) -> Self {
    let crc_li = data
      .chunks(chunk_size)
      .map(|chunk| {
        let mut h = Hasher::new();
        h.update(chunk);
        h.finalize()
      })
      .collect();
    Self { crc_li }
  }
}

impl Verify for CrcVerify {
  fn verify(&self, idx: usize, data: &[u8]) -> bool {
    let mut h = Hasher::new();
    h.update(data);
    self.crc_li.get(idx).copied() == Some(h.finalize())
  }
}

#[compio::test]
async fn test_multi_source_verify_failover_and_redirect() -> Void {
  let chunk_size = 64 * 1024;
  let total_size = 8 * chunk_size + 123;
  let data = gen_payload(total_size);
  let verify = CrcVerify::from_data(&data, chunk_size);

  let m0 = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  {
    let mut f = m0.fault_li.borrow_mut();
    f.resize(9, Fault::Ok);
    f[0] = Fault::Corrupt;
    f[2] = Fault::Status500;
    f[5] = Fault::Corrupt;
  }

  let m1 = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  {
    let mut f = m1.fault_li.borrow_mut();
    f.resize(9, Fault::Ok);
    f[0] = Fault::Status500;
    f[1] = Fault::Corrupt;
    f[3] = Fault::Chunked;
  }

  let mut m2 = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  m2.expect_path = Some("/cdn/real.bin?token=ok".to_owned());
  {
    let mut f = m2.fault_li.borrow_mut();
    f.resize(9, Fault::Ok);
    f[5] = Fault::Status500;
  }

  let (u0, _) = start_mirror(m0).await;
  let (u1, _) = start_mirror(m1).await;
  let (_, u2_addr) = start_mirror(m2).await;

  let mut m2_redir = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  m2_redir.redirect_to = Some(format!("http://{u2_addr}/cdn/real.bin?token=ok"));
  let (u2, _) = start_mirror(m2_redir).await;

  let dir = tempdir().expect("tempdir");
  let out = dir.path().join("archive.tar.gz");
  let part = part_path(&out);

  let got = Get::new([u0, u1, u2], &out)
    .chunk_size(chunk_size)
    .conc(4)
    .verify(verify)
    .run()
    .await
    .expect("download ok");

  assert_eq!(got, total_size as u64);
  assert!(!part.exists(), ".part file must be removed after rename");
  assert_eq!(read(&out)?, &*data);
  OK
}

#[compio::test]
async fn test_exhausted_all_sources() {
  let chunk_size = 64 * 1024;
  let total_size = 4 * chunk_size;
  let data = gen_payload(total_size);
  let verify = CrcVerify::from_data(&data, chunk_size);

  let mut hit_li = Vec::new();
  let mut url_li = Vec::new();
  for _ in 0..3 {
    let m = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
    {
      let mut f = m.fault_li.borrow_mut();
      f.resize(4, Fault::Ok);
      f[2] = Fault::Corrupt;
    }
    hit_li.push(Rc::clone(&m.req_li));
    let (u, _) = start_mirror(m).await;
    url_li.push(u);
  }

  let dir = tempdir().expect("tempdir");
  let out = dir.path().join("fail.bin");
  let err = Get::new(url_li, &out)
    .chunk_size(chunk_size)
    .conc(3)
    .verify(verify)
    .run()
    .await
    .expect_err("must fail on chunk 2");

  assert!(matches!(err, Error::Exhausted { idx: 2 }));
  let c2_start = (2 * chunk_size) as u64;
  for h in hit_li {
    assert!(
      h.borrow().iter().any(|&(s, _)| s == c2_start),
      "chunk 2 must have traversed every source"
    );
  }
}

#[compio::test]
async fn test_sparse_trailer_resume_auto_chunk() {
  // 4 MiB 文件在 conc=4 时，plan_chunk_size 自动规划为 256 KiB（共 16 块）
  let total_size = 4 * 1024 * 1024;
  let expected_chunk = plan_chunk_size(total_size as u64, 4);
  assert_eq!(expected_chunk, 256 * 1024);

  let data = gen_payload(total_size);
  let verify = Rc::new(CrcVerify::from_data(&data, expected_chunk));

  let m0 = MirrorOpt::healthy(Arc::clone(&data), expected_chunk);
  let m1 = MirrorOpt::healthy(Arc::clone(&data), expected_chunk);
  {
    let mut f0 = m0.fault_li.borrow_mut();
    let mut f1 = m1.fault_li.borrow_mut();
    f0.resize(16, Fault::Ok);
    f1.resize(16, Fault::Ok);
    f0[15] = Fault::Corrupt;
    f1[15] = Fault::Status500;
  }

  let fault0 = Rc::clone(&m0.fault_li);
  let fault1 = Rc::clone(&m1.fault_li);
  let req0_r1 = Rc::clone(&m0.req_li);
  let req1_r1 = Rc::clone(&m1.req_li);

  let (u0, _) = start_mirror(m0).await;
  let (u1, _) = start_mirror(m1).await;

  let dir = tempdir().expect("tempdir");
  let out = dir.path().join("model.safetensors");
  let part = part_path(&out);

  // 第一轮：末尾块 15 全源失败中断，其余已落盘分块进度写入尾部单张位图并保留 `.part`
  let err = Get::new([&u0, &u1], &out)
    .conc(4)
    .verify(Rc::clone(&verify))
    .run()
    .await
    .expect_err("round 1 fails at chunk 15");
  assert!(matches!(err, Error::Exhausted { idx: 15 }));
  assert!(!out.exists());
  assert!(part.exists(), ".part file must remain for resume");

  let part_len = StdFile::open(&part)
    .expect("open part")
    .metadata()
    .expect("meta")
    .len();
  assert_eq!(
    part_len,
    total_size as u64 + 2 + 24,
    ".part must equal payload + 2B bitmap + 24B footer"
  );

  // 统计第一轮请求后清空日志并修复源 15，准备第二轮续传
  let total_req_r1 = req0_r1.borrow().len() + req1_r1.borrow().len();
  assert!(total_req_r1 >= 12);
  req0_r1.borrow_mut().clear();
  req1_r1.borrow_mut().clear();
  fault0.borrow_mut()[15] = Fault::Ok;
  fault1.borrow_mut()[15] = Fault::Ok;

  // 第二轮改变并发度 .conc(8)，应自动沿用 .part 尾部记录的 256 KiB chunk_size 并跳过已落盘分块
  let got = Get::new([&u0, &u1], &out)
    .conc(8)
    .verify(verify)
    .run()
    .await
    .expect("round 2 resumes cleanly");

  assert_eq!(got, total_size as u64);
  assert!(!part.exists(), ".part file must be renamed away");
  let r2_li: Vec<(u64, u64)> = req0_r1
    .borrow()
    .iter()
    .chain(req1_r1.borrow().iter())
    .copied()
    .collect();
  assert!(
    r2_li.len() < 10,
    "resumed run must skip completed chunks, got {} requests",
    r2_li.len()
  );
  assert!(
    !r2_li.iter().any(|&(s, _)| s == 0),
    "already completed chunk 0 must not be re-requested on resume"
  );
  assert_eq!(read(&out).expect("read final"), &*data);
}

#[compio::test]
async fn test_work_stealing_and_endgame_racing() {
  let chunk_size = 64 * 1024;
  let n_chunk = 12;
  let total_size = chunk_size * n_chunk;
  let data = gen_payload(total_size);
  let verify = CrcVerify::from_data(&data, chunk_size);

  let fast = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  let mut slow = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  slow.delay = Duration::from_millis(450);

  let fast_req = Rc::clone(&fast.req_li);
  let slow_req = Rc::clone(&slow.req_li);

  let (u_fast, _) = start_mirror(fast).await;
  let (u_slow, _) = start_mirror(slow).await;

  let dir = tempdir().expect("tempdir");
  let out = dir.path().join("steal.bin");

  // 2 个并发协程分别对应 fast 与 slow：worker 1 在 slow 的第一个分块卡住 450ms；
  // worker 0 在 fast 上不仅跑完前半段，还通过二分窃取抢完后半段，最后对 slow 在途块发起尾盘竞速并胜出
  let t0 = Instant::now();
  let got = Get::new([u_fast, u_slow], &out)
    .size(total_size as u64)
    .chunk_size(chunk_size)
    .conc(2)
    .verify(verify)
    .run()
    .await
    .expect("steal + race must succeed");
  let elapsed = t0.elapsed();

  assert_eq!(got, total_size as u64);
  assert!(
    elapsed < Duration::from_millis(250),
    "fast source should steal & race tail chunk in <250ms, took {elapsed:?}"
  );
  assert_eq!(
    fast_req.borrow().len(),
    n_chunk,
    "fast mirror should end up serving all {n_chunk} chunks including stolen and endgame-raced ones"
  );
  assert!(
    slow_req.borrow().len() <= 2,
    "slow mirror should only be hit on initial assignment before being overtaken"
  );
  assert_eq!(read(&out).expect("read out"), &*data);
}

#[compio::test]
async fn test_part_lock_and_empty_and_chunk_plan() {
  assert_eq!(plan_chunk_size(0, 8), MIN_CHUNK);
  assert_eq!(plan_chunk_size(1024, 8), MIN_CHUNK);
  assert_eq!(plan_chunk_size(16 * 1024 * 1024, 8), 512 * 1024);
  assert_eq!(plan_chunk_size(64 * 1024 * 1024, 8), 2 * 1024 * 1024);
  assert_eq!(plan_chunk_size(10 * 1024 * 1024 * 1024, 8), MAX_CHUNK);

  let mut m_empty = MirrorOpt::healthy(Arc::from([]), MIN_CHUNK);
  m_empty.empty_416 = true;
  let (u_empty, _) = start_mirror(m_empty).await;

  let dir = tempdir().expect("tempdir");
  let empty_out = dir.path().join("empty.bin");
  let got = get([&u_empty], &empty_out).await.expect("empty file ok");
  assert_eq!(got, 0);
  assert_eq!(metadata(&empty_out).expect("meta").len(), 0);
  assert!(!part_path(&empty_out).exists());

  // 测试排他文件锁：持锁期间再次启动下载必须返回 Error::Locked 且不破坏已有 .part
  let locked_out = dir.path().join("locked.bin");
  let locked_part = part_path(&locked_out);
  let hold = StdFile::options()
    .read(true)
    .write(true)
    .create(true)
    .truncate(false)
    .open(&locked_part)
    .expect("create part");
  hold.try_lock().expect("acquire lock");

  let err = Get::new([&u_empty], &locked_out)
    .size(1024)
    .run()
    .await
    .expect_err("must reject locked part");
  assert!(matches!(err, Error::Locked));
  assert!(locked_part.exists());
}

#[compio::test]
async fn test_resume_edge_cases() {
  let chunk_size = 64 * 1024;
  let total_size = 10 * chunk_size + 77;
  let data = gen_payload(total_size);
  let verify = Rc::new(CrcVerify::from_data(&data, chunk_size));
  let m = MirrorOpt::healthy(Arc::clone(&data), chunk_size);
  let req = Rc::clone(&m.req_li);
  let (url, _) = start_mirror(m).await;

  let dir = tempdir().expect("tempdir");

  // 1. 构造 100% 完成态的 `.part`（11 块跨字节位图 0xff + 0x07 + 24B 尾标）：续传应 0 次分块请求直接截断重命名
  let done_out = dir.path().join("pre_done.bin");
  let done_part = part_path(&done_out);
  {
    let mut f = StdFile::options()
      .read(true)
      .write(true)
      .create(true)
      .truncate(true)
      .open(&done_part)
      .expect("create done part");
    f.write_all(&data).expect("write payload");
    f.write_all(&[0xff, 0x07]).expect("write full bitmap");
    f.write_all(&(total_size as u64).to_le_bytes())
      .expect("write size");
    f.write_all(&(chunk_size as u64).to_le_bytes())
      .expect("write chunk");
    f.write_all(b"MSRCGET1").expect("write magic");
  }
  let got = Get::new([&url], &done_out)
    .size(total_size as u64)
    .chunk_size(chunk_size)
    .verify(Rc::clone(&verify))
    .run()
    .await
    .expect("complete part finalizes directly");
  assert_eq!(got, total_size as u64);
  assert!(!done_part.exists());
  assert!(req.borrow().is_empty());
  assert_eq!(read(&done_out).expect("read done_out"), &*data);

  // 2. 破坏尾标 MAGIC：即使 `.part` 位图全 1，仍必须归零重建并完整重下
  let bad_out = dir.path().join("corrupt_trailer.bin");
  let bad_part = part_path(&bad_out);
  {
    let mut f = StdFile::options()
      .read(true)
      .write(true)
      .create(true)
      .truncate(true)
      .open(&bad_part)
      .expect("create bad part");
    f.write_all(&vec![0u8; total_size]).expect("write zero");
    f.write_all(&[0xff, 0x07]).expect("write stale bitmap");
    f.write_all(&(total_size as u64).to_le_bytes())
      .expect("write size");
    f.write_all(&(chunk_size as u64).to_le_bytes())
      .expect("write chunk");
    f.write_all(b"BADMAGIC").expect("write bad magic");
  }
  let got2 = Get::new([&url], &bad_out)
    .size(total_size as u64)
    .chunk_size(chunk_size)
    .verify(verify)
    .run()
    .await
    .expect("corrupt trailer rebuilds cleanly");
  assert_eq!(got2, total_size as u64);
  assert!(!bad_part.exists());
  assert_eq!(req.borrow().len(), 11);
  assert_eq!(read(&bad_out).expect("read bad_out"), &*data);

  // 3. 显式修改 `.chunk_size(...)` 且已有合法旧 `.part`：必须丢弃旧进度按新分块大小重建并完整重下
  req.borrow_mut().clear();
  let m_alt = MirrorOpt::healthy(Arc::clone(&data), chunk_size * 2);
  let alt_verify = CrcVerify::from_data(&data, chunk_size * 2);
  m_alt.fault_li.borrow_mut().resize(6, Fault::Ok);
  let req_alt = Rc::clone(&m_alt.req_li);
  let (url_alt, _) = start_mirror(m_alt).await;
  let chg_out = dir.path().join("changed_chunk.bin");
  let chg_part = part_path(&chg_out);
  {
    let mut f = StdFile::options()
      .read(true)
      .write(true)
      .create(true)
      .truncate(true)
      .open(&chg_part)
      .expect("create changed chunk part");
    f.write_all(&vec![0u8; total_size]).expect("write zero");
    f.write_all(&[0xff, 0x07]).expect("write stale bitmap");
    f.write_all(&(total_size as u64).to_le_bytes())
      .expect("write size");
    f.write_all(&(chunk_size as u64).to_le_bytes())
      .expect("write old chunk");
    f.write_all(b"MSRCGET1").expect("write magic");
  }
  let got3 = Get::new([&url_alt], &chg_out)
    .size(total_size as u64)
    .chunk_size(chunk_size * 2)
    .verify(alt_verify)
    .run()
    .await
    .expect("changed chunk size rebuilds cleanly");
  assert_eq!(got3, total_size as u64);
  assert!(!chg_part.exists());
  assert_eq!(req_alt.borrow().len(), 6);
  assert_eq!(read(&chg_out).expect("read chg_out"), &*data);
}
