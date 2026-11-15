use std::{cell::RefCell, mem::take, path::Path, time::Duration};

use compio::{BufResult, fs::File, io::AsyncWriteAtExt};
use futures_util::{
  future::join,
  stream::{FuturesUnordered, StreamExt},
};
use log::warn;

use crate::{
  conn::Conn,
  error::{Error, Result},
  http::{WorkerBuf, fetch_range, probe_size},
  resume::{OpenState, PartFile},
  sched::{MAX_RACER, NextCkpt, NextTask, Sched, Task},
  target::Target,
  verify::Verify,
};

/// 默认并发协程数
pub const CONC: usize = 8;
/// 默认单次网络请求与读取超时
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// 使用默认参数从多个镜像源并发下载文件到本地路径
pub async fn get<I, S>(url_li: I, path: impl AsRef<Path>) -> Result<u64>
where
  I: IntoIterator<Item = S>,
  S: AsRef<str>,
{
  Get::new(url_li, path).run().await
}

/// 可配置分块大小、并发数、超时与逐块校验器的多源下载器
pub struct Get<I, P, V = ()> {
  url_li: I,
  path: P,
  size: Option<u64>,
  chunk_size: Option<usize>,
  conc: usize,
  timeout: Duration,
  verify: V,
}

impl<I, P> Get<I, P, ()> {
  /// 构造多源下载任务
  pub fn new(url_li: I, path: P) -> Self {
    Self {
      url_li,
      path,
      size: None,
      chunk_size: None,
      conc: CONC,
      timeout: TIMEOUT,
      verify: (),
    }
  }
}

impl<I, P, V> Get<I, P, V> {
  /// 预设文件总字节数以跳过初始长度探测
  pub fn size(mut self, size: u64) -> Self {
    self.size = Some(size);
    self
  }

  /// 手动覆盖分块大小（字节）；未设置时按文件大小与并发数自动规划
  pub fn chunk_size(mut self, chunk_size: usize) -> Self {
    self.chunk_size = Some(chunk_size);
    self
  }

  /// 设置最大并发请求数
  pub fn conc(mut self, conc: usize) -> Self {
    self.conc = conc;
    self
  }

  /// 设置单次请求与读写的超时时间
  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// 传入实现了 [`Verify`] 的逐块校验器
  pub fn verify<V2: Verify>(self, verify: V2) -> Get<I, P, V2> {
    Get {
      url_li: self.url_li,
      path: self.path,
      size: self.size,
      chunk_size: self.chunk_size,
      conc: self.conc,
      timeout: self.timeout,
      verify,
    }
  }
}

impl<I, S, P, V> Get<I, P, V>
where
  I: IntoIterator<Item = S>,
  S: AsRef<str>,
  P: AsRef<Path>,
  V: Verify,
{
  /// 执行多源并发分块下载并返回总字节数；支持稀疏文件尾部断点续传、任务窃取与尾盘竞速
  pub async fn run(self) -> Result<u64> {
    if self.chunk_size == Some(0) {
      return Err(Error::ChunkSize);
    }

    let mut target_li = Vec::new();
    for raw in self.url_li {
      target_li.push(Target::parse(raw.as_ref())?);
    }
    if target_li.is_empty() {
      return Err(Error::EmptySrc);
    }

    let n_src = target_li.len();
    let mut conn_li: Box<[Vec<Conn>]> = (0..n_src).map(|_| Vec::new()).collect();
    let mut probe_buf = WorkerBuf::new(0);

    let total_size = match self.size {
      Some(sz) => sz,
      None => probe_any(&target_li, &mut conn_li, &mut probe_buf, self.timeout).await?,
    };

    let path = self.path.as_ref();
    if total_size == 0 {
      PartFile::finish_empty(path).await?;
      return Ok(0);
    }

    let conc = self.conc.max(1);
    let OpenState {
      part,
      chunk_size,
      total_chunk,
      done_bit_li,
      done_cnt,
    } = PartFile::open(path, total_size, self.chunk_size, conc).await?;

    if done_cnt == total_chunk {
      part.finalize().await?;
      return Ok(total_size);
    }

    let max_racer = total_chunk.saturating_mul(n_src.min(MAX_RACER));
    let n_worker = conc.min(max_racer).max(1);
    let bitmap_len = done_bit_li.len();

    let sched = RefCell::new(Sched::new(
      n_src,
      total_chunk,
      n_worker,
      conn_li,
      done_bit_li,
      done_cnt,
    ));
    let ctx = Ctx {
      sched: &sched,
      target_li: &target_li,
      file: &part.file,
      verify: &self.verify,
      total_size,
      chunk_size,
      dur: self.timeout,
    };

    let (w_res, c_res) = join(
      async {
        let res = run_workers(&ctx, n_worker).await;
        sched.borrow_mut().stop_ckpt();
        res
      },
      ckpt_loop(&sched, &part, bitmap_len),
    )
    .await;

    let sched_err = sched.borrow_mut().err.take();
    let err = match (w_res.and(c_res), sched_err) {
      (Err(e), _) | (Ok(()), Some(e)) => Some(e),
      (Ok(()), None) => None,
    };
    if let Some(err) = err {
      let dirty = {
        let s = sched.borrow();
        if s.flushing {
          Some((0, s.done_bit_li.clone()))
        } else if s.dirty_lo < s.dirty_hi {
          Some((s.dirty_lo, s.done_bit_li[s.dirty_lo..s.dirty_hi].to_vec()))
        } else {
          None
        }
      };
      part.abort_keep(dirty).await;
      return Err(err);
    }

    part.finalize().await?;
    Ok(total_size)
  }
}

async fn probe_any(
  target_li: &[Target],
  conn_li: &mut [Vec<Conn>],
  buf: &mut WorkerBuf,
  dur: Duration,
) -> Result<u64> {
  let mut last_err = None;
  for (i, target) in target_li.iter().enumerate() {
    match probe_size(target, None, buf, dur).await {
      Ok((sz, opt_conn)) => {
        if let Some(conn) = opt_conn {
          conn_li[i].push(conn);
        }
        return Ok(sz);
      }
      Err(err) => {
        warn!("probe {} failed: {err}", target.url);
        last_err = Some(err);
      }
    }
  }
  Err(last_err.unwrap_or(Error::Probe))
}

struct Ctx<'a, V> {
  sched: &'a RefCell<Sched>,
  target_li: &'a [Target],
  file: &'a File,
  verify: &'a V,
  total_size: u64,
  chunk_size: usize,
  dur: Duration,
}

async fn run_workers<V: Verify>(ctx: &Ctx<'_, V>, n_worker: usize) -> Result<()> {
  let mut fut_li = FuturesUnordered::new();
  for worker in 0..n_worker {
    fut_li.push(worker_loop(worker, ctx));
  }

  while let Some(res) = fut_li.next().await {
    res?;
  }
  Ok(())
}

async fn ckpt_loop(sched: &RefCell<Sched>, part: &PartFile, bitmap_len: usize) -> Result<()> {
  let mut bit_buf = Vec::with_capacity(bitmap_len);
  while let Some(off) = (NextCkpt {
    sched,
    buf: &mut bit_buf,
  })
  .await
  {
    let buf = take(&mut bit_buf);
    match part.write_bitmap(off, buf).await {
      Ok(back) => {
        bit_buf = back;
        sched.borrow_mut().finish_ckpt();
      }
      Err(err) => {
        sched.borrow_mut().abort_ckpt();
        return Err(err);
      }
    }
  }
  Ok(())
}

async fn worker_loop<V: Verify>(worker: usize, ctx: &Ctx<'_, V>) -> Result<()> {
  let mut buf = WorkerBuf::new(ctx.chunk_size);
  let mut pref = None;

  loop {
    let task = NextTask {
      sched: ctx.sched,
      worker,
      pref,
    }
    .await;

    let Task::Job {
      idx,
      id,
      src,
      conn,
      cancel,
    } = task
    else {
      return Ok(());
    };

    let start = idx as u64 * ctx.chunk_size as u64;
    let expected_len = (ctx.total_size - start).min(ctx.chunk_size as u64) as usize;

    let Some(res) = cancel
      .run(fetch_range(
        &ctx.target_li[src],
        conn,
        &mut buf,
        start,
        expected_len,
        ctx.total_size,
        ctx.dur,
      ))
      .await
    else {
      ctx.sched.borrow_mut().report_abort(idx, id);
      if pref == Some(src) {
        pref = None;
      }
      continue;
    };

    match res {
      Ok(mut opt_conn) => {
        if ctx.verify.verify(idx, &buf.data) {
          if !ctx
            .sched
            .borrow_mut()
            .claim_win(idx, id, src, &mut opt_conn)
          {
            pref = Some(src);
            continue;
          }

          let mut f = ctx.file;
          let data = take(&mut buf.data);
          let BufResult(write_res, back) = f.write_all_at(data, start).await;
          buf.data = back;
          write_res?;
          ctx.sched.borrow_mut().finish_win(idx, src, opt_conn);
          pref = Some(src);
        } else {
          warn!(
            "chunk {idx} failed verification from {}",
            ctx.target_li[src].url
          );
          ctx.sched.borrow_mut().report_fail(idx, id, src, opt_conn);
          if pref == Some(src) {
            pref = None;
          }
        }
      }
      Err(err) => {
        warn!("chunk {idx} failed from {}: {err}", ctx.target_li[src].url);
        ctx.sched.borrow_mut().report_fail(idx, id, src, None);
        if pref == Some(src) {
          pref = None;
        }
      }
    }
  }
}
