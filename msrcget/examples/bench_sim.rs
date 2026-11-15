//! 本机回环与限速仿真评测（内部调试用：cargo run --profile bench --example bench_sim）

#[path = "bench_common/mod.rs"]
mod bench_common;

use std::{
  env::temp_dir,
  fs::{create_dir_all, remove_dir_all},
  io::Write,
  net::SocketAddr,
  path::PathBuf,
  str::from_utf8,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use bench_common::{
  CrcVerify, KB, MB, bench_case, build_crc_li, crc_of, make_client, print_table, run_cyper_single,
  run_down, run_reqwest_single, run_ureq_single,
};
use memchr::memmem::find;
use msrcget::{Get, plan_chunk_size};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener as TokioListener, TcpStream as TokioStream},
  runtime::{Builder as TokioBuilder, Runtime as TokioRt},
  select, spawn as tokio_spawn,
  time::sleep as tokio_sleep,
};

#[derive(Clone)]
struct ServerCfg {
  data: Arc<Vec<u8>>,
  head_delay: Duration,
  slice_size: usize,
  slice_delay: Duration,
}

struct MockPool {
  url_li: Vec<String>,
  stop: Arc<AtomicBool>,
}

impl MockPool {
  fn url_ref_li(&self) -> Vec<&str> {
    self.url_li.iter().map(String::as_str).collect()
  }
}

impl Drop for MockPool {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
  }
}

fn gen_data(len: usize) -> Arc<Vec<u8>> {
  let mut buf = Vec::with_capacity(len);
  let mut x: u64 = 0x517c_c1b7_2722_0a95;
  for _ in 0..len {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    buf.push((x >> 16) as u8);
  }
  Arc::new(buf)
}

fn start_pool(srv_rt: &TokioRt, cfg_li: Vec<ServerCfg>) -> MockPool {
  let mut url_li = Vec::with_capacity(cfg_li.len());
  let stop = Arc::new(AtomicBool::new(false));

  for cfg in cfg_li {
    let stop_accept = Arc::clone(&stop);
    let addr: SocketAddr = srv_rt.block_on(async move {
      let listener = TokioListener::bind("127.0.0.1:0").await.expect("bind");
      let addr = listener.local_addr().expect("addr");
      tokio_spawn(async move {
        loop {
          if stop_accept.load(Ordering::Relaxed) {
            break;
          }
          let accept_res = select! {
            res = listener.accept() => res,
            _ = tokio_sleep(Duration::from_millis(200)) => continue,
          };
          let Ok((stream, _)) = accept_res else {
            break;
          };
          let _ = stream.set_nodelay(true);
          let conn_cfg = cfg.clone();
          tokio_spawn(serve_conn(stream, conn_cfg));
        }
      });
      addr
    });
    url_li.push(format!("http://{addr}/payload.bin"));
  }

  MockPool { url_li, stop }
}

fn parse_range(raw: &[u8], total: usize) -> Option<(usize, usize)> {
  let req = from_utf8(raw).ok()?;
  for line in req.lines() {
    if let Some((k, v)) = line.split_once(':')
      && k.trim().eq_ignore_ascii_case("range")
    {
      let spec = v.trim().strip_prefix("bytes=")?;
      let (s_s, e_s) = spec.split_once('-')?;
      let s: usize = s_s.trim().parse().ok()?;
      let e: usize = if e_s.trim().is_empty() {
        total.saturating_sub(1)
      } else {
        e_s.trim().parse().ok()?
      };
      return Some((s, e));
    }
  }
  None
}

async fn write_body_paced(
  stream: &mut TokioStream,
  body: &[u8],
  slice_size: usize,
  slice_delay: Duration,
) -> bool {
  if slice_delay.is_zero() || slice_size == 0 {
    return stream.write_all(body).await.is_ok();
  }
  for part in body.chunks(slice_size) {
    tokio_sleep(slice_delay).await;
    if stream.write_all(part).await.is_err() {
      return false;
    }
  }
  true
}

async fn serve_conn(mut stream: TokioStream, cfg: ServerCfg) {
  let mut req_buf = [0u8; 4096];
  let mut filled = 0usize;
  let mut head_buf = Vec::with_capacity(256);

  loop {
    let head_end = loop {
      if let Some(pos) = find(&req_buf[..filled], b"\r\n\r\n") {
        break pos + 4;
      }
      if filled == req_buf.len() {
        return;
      }
      let n = match stream.read(&mut req_buf[filled..]).await {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
      };
      filled += n;
    };

    let total = cfg.data.len();
    let range_opt = parse_range(&req_buf[..head_end], total);
    let tail = filled - head_end;
    req_buf.copy_within(head_end..filled, 0);
    filled = tail;

    head_buf.clear();
    if let Some((s, e)) = range_opt {
      if s >= total || e >= total || s > e {
        let _ = write!(
          head_buf,
          "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\n\r\n"
        );
        if stream.write_all(&head_buf).await.is_err() {
          return;
        }
        continue;
      }

      if !cfg.head_delay.is_zero() && !(s == 0 && e == 0) {
        tokio_sleep(cfg.head_delay).await;
      }

      let body = &cfg.data[s..=e];
      let _ = write!(
        head_buf,
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {s}-{e}/{total}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        body.len()
      );
      if stream.write_all(&head_buf).await.is_err() {
        return;
      }
      if !write_body_paced(
        &mut stream,
        body,
        cfg.slice_size,
        if s == 0 && e == 0 {
          Duration::ZERO
        } else {
          cfg.slice_delay
        },
      )
      .await
      {
        return;
      }
    } else {
      if !cfg.head_delay.is_zero() {
        tokio_sleep(cfg.head_delay).await;
      }
      let _ = write!(
        head_buf,
        "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: keep-alive\r\n\r\n"
      );
      if stream.write_all(&head_buf).await.is_err() {
        return;
      }
      if !write_body_paced(&mut stream, &cfg.data, cfg.slice_size, cfg.slice_delay).await {
        return;
      }
    }
  }
}

fn make_payload(size: usize) -> (Arc<Vec<u8>>, u32, usize, Arc<[u32]>) {
  let data = gen_data(size);
  let crc = crc_of(&data);
  let chunk = plan_chunk_size(size as u64, 8);
  let crc_li = build_crc_li(&data, chunk);
  (data, crc, chunk, crc_li)
}

fn main() {
  let srv_rt = TokioBuilder::new_multi_thread()
    .worker_threads(4)
    .enable_all()
    .build()
    .expect("server tokio rt");
  let (cli_tokio, cli_compio, cyper_cli, reqwest_cli, ureq_agent) = make_client(None);

  let dir = temp_dir().join("msrcget_bench");
  create_dir_all(&dir).expect("create dir");
  let out: PathBuf = dir.join("bench_target.bin");

  // 场景 1: 单源回环 64 MB（无人工延迟，衡量各库纯协议栈与磁盘落盘效率）
  let size1 = 64 * MB;
  let (data1, crc1, chunk1, crc_li1) = make_payload(size1);
  let pool1 = start_pool(
    &srv_rt,
    vec![ServerCfg {
      data: data1,
      head_delay: Duration::ZERO,
      slice_size: 0,
      slice_delay: Duration::ZERO,
    }],
  );
  let url_li1 = pool1.url_ref_li();

  let row_li1 = [
    bench_case("msrcget", &out, size1, crc1, 1, 3, |p| {
      cli_compio
        .block_on(Get::new(&url_li1, p).size(size1 as u64).run())
        .expect("msrcget");
    }),
    bench_case("msrcget + CRC32", &out, size1, crc1, 1, 3, |p| {
      cli_compio
        .block_on(
          Get::new(&url_li1, p)
            .size(size1 as u64)
            .chunk_size(chunk1)
            .verify(CrcVerify(Arc::clone(&crc_li1)))
            .run(),
        )
        .expect("msrcget crc");
    }),
    bench_case("down", &out, size1, crc1, 1, 3, |p| {
      run_down(&cli_tokio, &url_li1, p);
    }),
    bench_case("cyper", &out, size1, crc1, 1, 3, |p| {
      run_cyper_single(&cli_compio, &cyper_cli, url_li1[0], p);
    }),
    bench_case("reqwest", &out, size1, crc1, 1, 3, |p| {
      run_reqwest_single(&cli_tokio, &reqwest_cli, url_li1[0], p);
    }),
    bench_case("ureq", &out, size1, crc1, 1, 3, |p| {
      run_ureq_single(&ureq_agent, url_li1[0], p);
    }),
  ];
  print_table("场景 1: 单源回环", size1, &row_li1);
  drop(pool1);

  // 场景 2: 4 源限速聚合 16 MB（单连接限速 8 MB/s + 10ms RTT）
  let size2 = 16 * MB;
  let (data2, crc2, chunk2, crc_li2) = make_payload(size2);
  let mut cfg_li2 = Vec::with_capacity(4);
  for _ in 0..4 {
    cfg_li2.push(ServerCfg {
      data: Arc::clone(&data2),
      head_delay: Duration::from_millis(10),
      slice_size: 128 * KB,
      slice_delay: Duration::from_millis(16),
    });
  }
  let pool2 = start_pool(&srv_rt, cfg_li2);
  let url_li2 = pool2.url_ref_li();

  let row_li2 = [
    bench_case("msrcget (4 源,8 并发)", &out, size2, crc2, 1, 3, |p| {
      cli_compio
        .block_on(Get::new(&url_li2, p).conc(8).run())
        .expect("msrcget");
    }),
    bench_case("msrcget + CRC32", &out, size2, crc2, 1, 3, |p| {
      cli_compio
        .block_on(
          Get::new(&url_li2, p)
            .conc(8)
            .chunk_size(chunk2)
            .verify(CrcVerify(Arc::clone(&crc_li2)))
            .run(),
        )
        .expect("msrcget crc");
    }),
    bench_case("down (4 源)", &out, size2, crc2, 1, 3, |p| {
      run_down(&cli_tokio, &url_li2, p);
    }),
    bench_case("cyper (单源)", &out, size2, crc2, 0, 2, |p| {
      run_cyper_single(&cli_compio, &cyper_cli, url_li2[0], p);
    }),
    bench_case("reqwest (单源)", &out, size2, crc2, 0, 2, |p| {
      run_reqwest_single(&cli_tokio, &reqwest_cli, url_li2[0], p);
    }),
    bench_case("ureq (单源)", &out, size2, crc2, 0, 2, |p| {
      run_ureq_single(&ureq_agent, url_li2[0], p);
    }),
  ];
  print_table("场景 2: 4 源限速聚合（每连接 8 MB/s）", size2, &row_li2);
  drop(pool2);

  // 场景 3: 4 源异构卡顿 8 MB（3 个正常节点 8 MB/s + 1 个慢节点 1200ms 延迟与 2 MB/s），验证任务窃取与尾盘竞速
  let size3 = 8 * MB;
  let (data3, crc3, chunk3, crc_li3) = make_payload(size3);
  let mut cfg_li3 = Vec::with_capacity(4);
  for _ in 0..3 {
    cfg_li3.push(ServerCfg {
      data: Arc::clone(&data3),
      head_delay: Duration::from_millis(5),
      slice_size: 128 * KB,
      slice_delay: Duration::from_millis(16),
    });
  }
  cfg_li3.push(ServerCfg {
    data: data3,
    head_delay: Duration::from_millis(1200),
    slice_size: 128 * KB,
    slice_delay: Duration::from_millis(64),
  });
  let pool3 = start_pool(&srv_rt, cfg_li3);
  let url_li3 = pool3.url_ref_li();

  let row_li3 = [
    bench_case(
      "msrcget (窃取+尾盘竞速)",
      &out,
      size3,
      crc3,
      1,
      3,
      |p| {
        cli_compio
          .block_on(
            Get::new(&url_li3, p)
              .conc(8)
              .chunk_size(chunk3)
              .verify(CrcVerify(Arc::clone(&crc_li3)))
              .run(),
          )
          .expect("msrcget");
      },
    ),
    bench_case("down (4 源含慢节点)", &out, size3, crc3, 0, 2, |p| {
      run_down(&cli_tokio, &url_li3, p);
    }),
    bench_case("reqwest (单源正常)", &out, size3, crc3, 0, 1, |p| {
      run_reqwest_single(&cli_tokio, &reqwest_cli, url_li3[0], p);
    }),
    bench_case("reqwest (单源慢节点)", &out, size3, crc3, 0, 1, |p| {
      run_reqwest_single(&cli_tokio, &reqwest_cli, url_li3[3], p);
    }),
  ];
  print_table("场景 3: 3 快源 + 1 慢节点卡顿", size3, &row_li3);
  drop(pool3);

  let _ = remove_dir_all(&dir);
}
