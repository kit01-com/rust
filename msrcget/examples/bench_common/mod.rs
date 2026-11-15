use std::{
  fs::{File as StdFile, metadata, remove_file},
  io::{Read, Write},
  mem::take,
  path::Path,
  sync::Arc,
  time::{Duration, Instant},
};

use compio::{
  BufResult, fs::File as CompioFile, io::AsyncWriteAtExt, runtime::Runtime as CompioRt,
  rustls as compio_tls,
};
use crc32fast::Hasher;
use cyper::Client as CyperClient;
use down::down;
use futures_util::StreamExt;
use msrcget::{Verify, part_path};
use reqwest::Client as ReqwestClient;
use tokio::{
  fs::File as TokioFile,
  io::AsyncWriteExt,
  runtime::{Builder as TokioBuilder, Runtime as TokioRt},
};
use ureq::{Agent, config::Config};

use self::compio_tls::crypto::ring::default_provider;

pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;
pub const UA: &str = "msrcget";

pub struct CrcVerify(pub Arc<[u32]>);

impl Verify for CrcVerify {
  fn verify(&self, idx: usize, data: &[u8]) -> bool {
    self.0.get(idx).copied() == Some(crc_of(data))
  }
}

pub fn crc_of(buf: &[u8]) -> u32 {
  let mut h = Hasher::new();
  h.update(buf);
  h.finalize()
}

pub fn build_crc_li(data: &[u8], chunk_size: usize) -> Arc<[u32]> {
  data.chunks(chunk_size).map(crc_of).collect()
}

/// 流式校验输出文件长度与 CRC32，避免整文件读入内存
pub fn verify_out(path: &Path, expected_len: usize, expected_crc: u32) {
  let meta = metadata(path).expect("output metadata");
  assert_eq!(meta.len(), expected_len as u64, "size mismatch");

  let mut file = StdFile::open(path).expect("open output");
  let mut hasher = Hasher::new();
  let mut buf = [0u8; 64 * KB];
  loop {
    let n = file.read(&mut buf).expect("read output");
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  assert_eq!(hasher.finalize(), expected_crc, "crc mismatch");
}

pub fn clean_path(path: &Path) {
  let _ = remove_file(path);
  let _ = remove_file(part_path(path));
}

fn median_sec(mut sec_li: Vec<f64>) -> f64 {
  if sec_li.is_empty() {
    return 0.0;
  }
  let mid = sec_li.len() / 2;
  let (_, med, _) = sec_li.select_nth_unstable_by(mid, f64::total_cmp);
  *med
}

pub fn bench_case<'a, F>(
  name: &'a str,
  out_path: &Path,
  expected_len: usize,
  expected_crc: u32,
  warmup: usize,
  iter: usize,
  mut run_fn: F,
) -> (&'a str, f64)
where
  F: FnMut(&Path),
{
  for _ in 0..warmup {
    clean_path(out_path);
    run_fn(out_path);
    verify_out(out_path, expected_len, expected_crc);
  }
  let mut sec_li = Vec::with_capacity(iter);
  for _ in 0..iter {
    clean_path(out_path);
    let t0 = Instant::now();
    run_fn(out_path);
    let sec = t0.elapsed().as_secs_f64();
    verify_out(out_path, expected_len, expected_crc);
    sec_li.push(sec);
  }
  clean_path(out_path);
  let med_sec = median_sec(sec_li);
  let mb_per_s = if med_sec > 0.0 {
    (expected_len as f64 / MB as f64) / med_sec
  } else {
    0.0
  };
  (name, mb_per_s)
}

pub fn print_table(title: &str, byte_len: usize, row_li: &[(&str, f64)]) {
  if byte_len.is_multiple_of(MB) {
    println!("\n=== {title} ({} MB) ===", byte_len / MB);
  } else {
    println!("\n=== {title} ({:.1} MB) ===", byte_len as f64 / MB as f64);
  }
  println!("{:<22} | {:>11}", "库 / 方案", "吞吐 (MB/s)");
  println!("{:-<22}-|-{:-<11}", "", "");
  for &(name, mb_s) in row_li {
    println!("{name:<22} | {mb_s:>11.1}");
  }
}

pub fn make_client(
  timeout: Option<Duration>,
) -> (TokioRt, CompioRt, CyperClient, ReqwestClient, Agent) {
  let _ = default_provider().install_default();
  let cli_tokio = TokioBuilder::new_multi_thread()
    .worker_threads(4)
    .enable_all()
    .build()
    .expect("client tokio rt");
  let cli_compio = CompioRt::new().expect("compio rt");
  let cyper_cli = CyperClient::new().expect("cyper client");

  let mut req_builder = ReqwestClient::builder().user_agent(UA);
  if let Some(dur) = timeout {
    req_builder = req_builder.timeout(dur);
  }
  let reqwest_cli = req_builder.build().expect("reqwest client");

  let ureq_agent: Agent = Config::builder()
    .user_agent(UA)
    .timeout_global(timeout)
    .build()
    .into();

  (cli_tokio, cli_compio, cyper_cli, reqwest_cli, ureq_agent)
}

pub fn run_cyper_single(rt: &CompioRt, client: &CyperClient, url: &str, out: &Path) {
  rt.block_on(async {
    let resp = client
      .get(url)
      .expect("req")
      .header("user-agent", UA)
      .expect("ua")
      .send()
      .await
      .expect("send");
    let status = resp.status();
    assert!(status.is_success(), "cyper status {status} for {url}");
    let mut file = CompioFile::create(out).await.expect("create");
    let mut stream = resp.bytes_stream();
    let mut pos = 0u64;
    let mut buf = Vec::with_capacity(128 * KB);
    while let Some(item) = stream.next().await {
      let chunk = item.expect("chunk");
      let len = chunk.len() as u64;
      buf.clear();
      buf.extend_from_slice(&chunk);
      let BufResult(res, back) = file.write_all_at(take(&mut buf), pos).await;
      res.expect("write");
      buf = back;
      pos += len;
    }
    file.sync_data().await.expect("sync");
    file.close().await.expect("close");
  });
}

pub fn run_reqwest_single(rt: &TokioRt, client: &ReqwestClient, url: &str, out: &Path) {
  rt.block_on(async {
    let resp = client.get(url).send().await.expect("send");
    let status = resp.status();
    assert!(status.is_success(), "reqwest status {status} for {url}");
    let mut file = TokioFile::create(out).await.expect("create");
    let mut stream = resp.bytes_stream();
    while let Some(item) = stream.next().await {
      let chunk = item.expect("chunk");
      file.write_all(&chunk).await.expect("write");
    }
    file.flush().await.expect("flush");
    file.sync_data().await.expect("sync");
  });
}

pub fn run_ureq_single(agent: &Agent, url: &str, out: &Path) {
  let resp = agent.get(url).call().expect("call");
  let mut reader = resp.into_body().into_reader();
  let mut file = StdFile::create(out).expect("create");
  let mut buf = [0u8; 64 * KB];
  loop {
    let n = reader.read(&mut buf).expect("read");
    if n == 0 {
      break;
    }
    file.write_all(&buf[..n]).expect("write");
  }
  file.flush().expect("flush");
  file.sync_data().expect("sync");
}

pub fn run_down(rt: &TokioRt, url_li: &[&str], out: &Path) {
  rt.block_on(async {
    let rx = down(url_li.iter().copied(), out).await.expect("down start");
    while rx.recv().await.is_ok() {}
  });
}
