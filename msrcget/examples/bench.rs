//! 公网真实 HTTPS 多镜像源性能评测（cargo run --profile bench --example bench）

#[path = "bench_common/mod.rs"]
mod bench_common;

use std::{
  env::temp_dir,
  fs::{create_dir_all, read, remove_dir_all},
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};

use bench_common::{
  CrcVerify, bench_case, build_crc_li, clean_path, crc_of, make_client, print_table,
  run_cyper_single, run_down, run_reqwest_single, run_ureq_single,
};
use compio::runtime::Runtime as CompioRt;
use cyper::Client as CyperClient;
use msrcget::{Get, get, plan_chunk_size};
use reqwest::Client as ReqwestClient;
use tokio::runtime::Runtime as TokioRt;
use ureq::Agent;

const CONC: usize = 8;
const ITER: usize = 3;
const REQ_TIMEOUT: Duration = Duration::from_secs(60);

const SIZE_1: usize = 29_893_360;
const URL_LI_1: [&str; 5] = [
  "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release/v22.14.0/node-v22.14.0-linux-x64.tar.xz",
  "https://mirrors.ustc.edu.cn/node/v22.14.0/node-v22.14.0-linux-x64.tar.xz",
  "https://mirrors.huaweicloud.com/nodejs/v22.14.0/node-v22.14.0-linux-x64.tar.xz",
  "https://mirrors.cloud.tencent.com/nodejs-release/v22.14.0/node-v22.14.0-linux-x64.tar.xz",
  "https://registry.npmmirror.com/-/binary/node/v22.14.0/node-v22.14.0-linux-x64.tar.xz",
];

const SIZE_2: usize = 8_771_344;
const URL_LI_2: [&str; 5] = [
  "https://mirrors.aliyun.com/nodejs-release/v22.14.0/node-v22.14.0-headers.tar.gz",
  "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release/v22.14.0/node-v22.14.0-headers.tar.gz",
  "https://mirrors.huaweicloud.com/nodejs/v22.14.0/node-v22.14.0-headers.tar.gz",
  "https://mirrors.cloud.tencent.com/nodejs-release/v22.14.0/node-v22.14.0-headers.tar.gz",
  "https://nodejs.org/dist/v22.14.0/node-v22.14.0-headers.tar.gz",
];

struct Scenario<'a> {
  title: &'a str,
  url_li: &'a [&'a str],
  expected_len: usize,
}

/// 预先下载基线并生成整包 CRC32 与逐块校验表
fn prepare_baseline(
  rt: &CompioRt,
  url_li: &[&str],
  out: &Path,
  expected_len: usize,
) -> (u32, usize, Arc<[u32]>) {
  clean_path(out);
  let got = rt.block_on(get(url_li, out)).expect("baseline get");
  assert_eq!(got, expected_len as u64, "baseline size mismatch");
  let raw = read(out).expect("read baseline");
  assert_eq!(raw.len(), expected_len, "baseline raw len mismatch");
  clean_path(out);

  let expected_crc = crc_of(&raw);
  let chunk_size = plan_chunk_size(expected_len as u64, CONC);
  (expected_crc, chunk_size, build_crc_li(&raw, chunk_size))
}

fn run_scenario(
  sc: &Scenario<'_>,
  out: &Path,
  cli_compio: &CompioRt,
  cli_tokio: &TokioRt,
  cyper_cli: &CyperClient,
  reqwest_cli: &ReqwestClient,
  ureq_agent: &Agent,
) {
  let (crc, chunk_size, crc_li) = prepare_baseline(cli_compio, sc.url_li, out, sc.expected_len);
  let row_li = [
    bench_case(
      "msrcget (5 源,8 并发)",
      out,
      sc.expected_len,
      crc,
      0,
      ITER,
      |p| {
        cli_compio
          .block_on(Get::new(sc.url_li, p).conc(CONC).run())
          .expect("msrcget");
      },
    ),
    bench_case("msrcget + CRC32", out, sc.expected_len, crc, 0, ITER, |p| {
      cli_compio
        .block_on(
          Get::new(sc.url_li, p)
            .conc(CONC)
            .chunk_size(chunk_size)
            .verify(CrcVerify(Arc::clone(&crc_li)))
            .run(),
        )
        .expect("msrcget crc");
    }),
    bench_case("down (5 源)", out, sc.expected_len, crc, 0, ITER, |p| {
      run_down(cli_tokio, sc.url_li, p);
    }),
    bench_case("cyper (单源)", out, sc.expected_len, crc, 0, ITER, |p| {
      run_cyper_single(cli_compio, cyper_cli, sc.url_li[0], p);
    }),
    bench_case(
      "reqwest (单源)",
      out,
      sc.expected_len,
      crc,
      0,
      ITER,
      |p| {
        run_reqwest_single(cli_tokio, reqwest_cli, sc.url_li[0], p);
      },
    ),
    bench_case("ureq (单源)", out, sc.expected_len, crc, 0, ITER, |p| {
      run_ureq_single(ureq_agent, sc.url_li[0], p);
    }),
  ];
  print_table(sc.title, sc.expected_len, &row_li);
}

fn main() {
  let (cli_tokio, cli_compio, cyper_cli, reqwest_cli, ureq_agent) = make_client(Some(REQ_TIMEOUT));

  let dir = temp_dir().join("msrcget_bench_real");
  create_dir_all(&dir).expect("create dir");
  let out: PathBuf = dir.join("bench_target.bin");

  let scenario_li = [
    Scenario {
      title: "场景 1: 多公共镜像聚合下载",
      url_li: &URL_LI_1,
      expected_len: SIZE_1,
    },
    Scenario {
      title: "场景 2: 快慢源混合下载",
      url_li: &URL_LI_2,
      expected_len: SIZE_2,
    },
  ];

  for sc in &scenario_li {
    run_scenario(
      sc,
      &out,
      &cli_compio,
      &cli_tokio,
      &cyper_cli,
      &reqwest_cli,
      &ureq_agent,
    );
  }

  let _ = remove_dir_all(&dir);
}
