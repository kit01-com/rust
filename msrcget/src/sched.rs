use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  future::{Future, poll_fn},
  pin::{Pin, pin},
  rc::Rc,
  task::{Context, Poll, Waker},
};

use gxhash::HashMap;

use crate::{
  conn::Conn,
  error::Error,
  resume::{bit_get, bit_set},
};

/// 自动分块下限：64 KiB
pub const MIN_CHUNK: usize = 64 * 1024;
/// 自动分块上限：4 MiB
pub const MAX_CHUNK: usize = 4 * 1024 * 1024;

const TARGET_CHUNK_PER_CONC: u64 = 4;
pub(crate) const MAX_RACER: usize = 4;

/// 根据文件总字节数与并发数自动规划分块大小（2 的幂，范围 `MIN_CHUNK..=MAX_CHUNK`）
pub const fn plan_chunk_size(file_size: u64, conc: usize) -> usize {
  let c = if conc == 0 { 1 } else { conc } as u64;
  let target_n = c.saturating_mul(TARGET_CHUNK_PER_CONC);
  let raw = file_size / target_n;
  let clamped = if raw < MIN_CHUNK as u64 {
    MIN_CHUNK
  } else if raw > MAX_CHUNK as u64 {
    MAX_CHUNK
  } else {
    raw as usize
  };
  1usize << (usize::BITS - 1 - clamped.leading_zeros())
}

struct CancelState {
  aborted: Cell<bool>,
  waker: Cell<Option<Waker>>,
}

/// 单线程零分配轻量取消令牌，用于在尾盘竞速中及时中断慢连接
#[derive(Clone)]
pub(crate) struct Cancel(Rc<CancelState>);

impl Cancel {
  pub(crate) fn new() -> Self {
    Self(Rc::new(CancelState {
      aborted: Cell::new(false),
      waker: Cell::new(None),
    }))
  }

  #[inline]
  pub(crate) fn abort(&self) {
    if !self.0.aborted.replace(true)
      && let Some(w) = self.0.waker.take()
    {
      w.wake();
    }
  }

  /// 执行异步操作；若同分块已有赢家通过校验则立即中断并返回 `None`
  pub(crate) async fn run<F: Future>(&self, fut: F) -> Option<F::Output> {
    if self.0.aborted.get() {
      return None;
    }
    let mut fut = pin!(fut);
    poll_fn(|cx| {
      let prev = self.0.waker.take();
      if self.0.aborted.get() {
        return Poll::Ready(None);
      }
      if let Poll::Ready(out) = fut.as_mut().poll(cx) {
        return Poll::Ready(Some(out));
      }
      if self.0.aborted.get() {
        return Poll::Ready(None);
      }
      let waker = match prev {
        Some(w) if w.will_wake(cx.waker()) => w,
        _ => cx.waker().clone(),
      };
      self.0.waker.set(Some(waker));
      Poll::Pending
    })
    .await
  }
}

/// 单块已失败源位图（<= 64 源零堆分配，> 64 源按需扩展）
enum SrcBit {
  Small(u64),
  Large(Box<[u64]>),
}

impl SrcBit {
  fn new(n_src: usize) -> Self {
    if n_src <= 64 {
      Self::Small(0)
    } else {
      Self::Large(vec![0; n_src.div_ceil(64)].into_boxed_slice())
    }
  }

  #[inline]
  fn has(&self, src: usize) -> bool {
    match self {
      Self::Small(m) => (m >> src) & 1 != 0,
      Self::Large(li) => (li[src >> 6] >> (src & 63)) & 1 != 0,
    }
  }

  #[inline]
  fn set(&mut self, src: usize) {
    match self {
      Self::Small(m) => *m |= 1u64 << src,
      Self::Large(li) => li[src >> 6] |= 1u64 << (src & 63),
    }
  }

  fn all_set(&self, n_src: usize) -> bool {
    match self {
      Self::Small(m) => {
        let mask = if n_src == 64 {
          u64::MAX
        } else {
          (1u64 << n_src) - 1
        };
        (*m & mask) == mask
      }
      Self::Large(li) => {
        let full = n_src >> 6;
        if li[..full].iter().any(|&w| w != u64::MAX) {
          return false;
        }
        let rem = n_src & 63;
        if rem == 0 {
          return true;
        }
        let mask = (1u64 << rem) - 1;
        (li[full] & mask) == mask
      }
    }
  }
}

#[derive(Clone, Copy)]
struct Seg {
  cur: usize,
  end: usize,
}

impl Seg {
  #[inline]
  fn len(self) -> usize {
    self.end.saturating_sub(self.cur)
  }

  fn trim_done(&mut self, done_bit_li: &[u8]) {
    while self.cur < self.end && bit_get(done_bit_li, self.cur) {
      self.cur += 1;
    }
    while self.cur < self.end && bit_get(done_bit_li, self.end - 1) {
      self.end -= 1;
    }
  }

  fn pop_front(&mut self, done_bit_li: &[u8]) -> Option<usize> {
    while self.cur < self.end {
      let idx = self.cur;
      self.cur += 1;
      if !bit_get(done_bit_li, idx) {
        return Some(idx);
      }
    }
    None
  }
}

struct Racer {
  id: u64,
  src: usize,
  cancel: Cancel,
}

struct InFlight {
  writing: bool,
  racer_li: Vec<Racer>,
}

impl InFlight {
  fn has_src(&self, src: usize) -> bool {
    self.racer_li.iter().any(|r| r.src == src)
  }
}

/// 调度器分配给工作协程的下一步指令
pub(crate) enum Task {
  Done,
  Wait,
  Job {
    idx: usize,
    id: u64,
    src: usize,
    conn: Option<Conn>,
    cancel: Cancel,
  },
}

/// 块级多源并发调度器：支持断点跳块、故障源遍历、二分任务窃取与尾盘多协程竞速
pub(crate) struct Sched {
  n_src: usize,
  total_chunk: usize,
  pub(crate) done_bit_li: Vec<u8>,
  pub(crate) done_cnt: usize,
  pub(crate) dirty_lo: usize,
  pub(crate) dirty_hi: usize,
  pub(crate) flushing: bool,
  stop_ckpt: bool,
  ckpt_aborted: bool,
  ckpt_waker: Option<Waker>,
  seg_li: Box<[Seg]>,
  retry_q: VecDeque<usize>,
  tried_map: HashMap<usize, SrcBit>,
  in_flight: HashMap<usize, InFlight>,
  next_id: u64,
  fail_li: Box<[u32]>,
  conn_li: Box<[Vec<Conn>]>,
  waker_li: Box<[Option<Waker>]>,
  pub(crate) err: Option<Error>,
}

impl Sched {
  pub(crate) fn new(
    n_src: usize,
    total_chunk: usize,
    n_worker: usize,
    conn_li: Box<[Vec<Conn>]>,
    done_bit_li: Vec<u8>,
    done_cnt: usize,
  ) -> Self {
    let mut seg_li: Box<[Seg]> = (0..n_src)
      .map(|s| {
        let cur = ((s as u64 * total_chunk as u64) / n_src as u64) as usize;
        let end = (((s + 1) as u64 * total_chunk as u64) / n_src as u64) as usize;
        Seg { cur, end }
      })
      .collect();
    for seg in &mut seg_li {
      seg.trim_done(&done_bit_li);
    }

    Self {
      n_src,
      total_chunk,
      done_bit_li,
      done_cnt,
      dirty_lo: usize::MAX,
      dirty_hi: 0,
      flushing: false,
      stop_ckpt: false,
      ckpt_aborted: false,
      ckpt_waker: None,
      seg_li,
      retry_q: VecDeque::new(),
      tried_map: HashMap::default(),
      in_flight: HashMap::default(),
      next_id: 0,
      fail_li: vec![0; n_src].into_boxed_slice(),
      conn_li,
      waker_li: vec![None; n_worker].into_boxed_slice(),
      err: None,
    }
  }

  /// 所有下载工作协程退出后通知位图刷盘协程立刻停止
  pub(crate) fn stop_ckpt(&mut self) {
    self.stop_ckpt = true;
    if let Some(w) = self.ckpt_waker.take() {
      w.wake();
    }
  }

  /// 位图刷盘协程因磁盘写入异常退出时，立即中止所有工作协程
  pub(crate) fn abort_ckpt(&mut self) {
    self.ckpt_aborted = true;
    self.abort_all();
    self.wake_all();
  }

  /// 异步位图写盘完成后清除刷新标记
  #[inline]
  pub(crate) fn finish_ckpt(&mut self) {
    self.flushing = false;
  }

  /// 在写入磁盘前竞价夺标；同块先通过校验者获胜并立刻中止其他竞速协程，落败者归还连接
  pub(crate) fn claim_win(
    &mut self,
    idx: usize,
    id: u64,
    src: usize,
    conn: &mut Option<Conn>,
  ) -> bool {
    self.fail_li[src] = 0;
    if bit_get(&self.done_bit_li, idx) {
      self.remove_racer_only(idx, id);
      if let Some(c) = conn.take() {
        self.conn_li[src].push(c);
      }
      return false;
    }
    let Some(inf) = self.in_flight.get_mut(&idx) else {
      if let Some(c) = conn.take() {
        self.conn_li[src].push(c);
      }
      return false;
    };
    if inf.writing || !inf.racer_li.iter().any(|r| r.id == id) {
      inf.racer_li.retain(|r| r.id != id);
      if let Some(c) = conn.take() {
        self.conn_li[src].push(c);
      }
      return false;
    }
    inf.writing = true;
    for r in &inf.racer_li {
      if r.id != id {
        r.cancel.abort();
      }
    }
    true
  }

  /// 赢家完成磁盘落盘后提交位图、清理状态并按需触发刷盘
  pub(crate) fn finish_win(&mut self, idx: usize, src: usize, conn: Option<Conn>) {
    self.fail_li[src] = 0;
    if let Some(c) = conn {
      self.conn_li[src].push(c);
    }
    self.in_flight.remove(&idx);
    self.tried_map.remove(&idx);
    if !bit_get(&self.done_bit_li, idx) {
      bit_set(&mut self.done_bit_li, idx);
      self.done_cnt += 1;
      let byte_idx = idx >> 3;
      self.dirty_lo = self.dirty_lo.min(byte_idx);
      self.dirty_hi = self.dirty_hi.max(byte_idx + 1);
    }
    if self.done_cnt == self.total_chunk {
      self.abort_all();
      self.wake_all();
      if let Some(w) = self.ckpt_waker.take() {
        w.wake();
      }
    } else if !self.flushing
      && self.dirty_lo < self.dirty_hi
      && let Some(w) = self.ckpt_waker.take()
    {
      w.wake();
    }
  }

  /// 竞速失败且未胜出的协程或中途非赢家报告单次源尝试失败
  pub(crate) fn report_fail(&mut self, idx: usize, id: u64, src: usize, conn: Option<Conn>) {
    if let Some(c) = conn {
      self.conn_li[src].push(c);
    }
    if self.ckpt_aborted || self.err.is_some() || bit_get(&self.done_bit_li, idx) {
      self.remove_racer_only(idx, id);
      return;
    }

    let writing = if let Some(inf) = self.in_flight.get_mut(&idx) {
      inf.racer_li.retain(|r| r.id != id);
      inf.writing
    } else {
      return;
    };

    if writing {
      return;
    }

    self.fail_li[src] = self.fail_li[src].saturating_add(1);
    let tried = self
      .tried_map
      .entry(idx)
      .or_insert_with(|| SrcBit::new(self.n_src));
    tried.set(src);

    if tried.all_set(self.n_src) {
      if let Some(inf) = self.in_flight.remove(&idx) {
        for r in inf.racer_li {
          r.cancel.abort();
        }
      }
      self.err = Some(Error::Exhausted { idx });
      self.abort_all();
      self.wake_all();
      return;
    }

    let mut need_requeue = false;
    if let Some(inf) = self.in_flight.get_mut(&idx) {
      let mut drop_li = Vec::new();
      inf.racer_li.retain(|r| {
        if r.src == src {
          drop_li.push(r.cancel.clone());
          false
        } else {
          true
        }
      });
      for c in drop_li {
        c.abort();
      }
      need_requeue = inf.racer_li.is_empty();
    }

    if need_requeue {
      self.in_flight.remove(&idx);
      self.retry_q.push_back(idx);
    }
    self.wake_all();
  }

  /// 处理被 `Cancel::abort` 主动中断的竞速尝试（不视为源失败）
  pub(crate) fn report_abort(&mut self, idx: usize, id: u64) {
    if self.ckpt_aborted || self.err.is_some() || bit_get(&self.done_bit_li, idx) {
      self.remove_racer_only(idx, id);
      return;
    }

    let empty_and_idle = if let Some(inf) = self.in_flight.get_mut(&idx) {
      inf.racer_li.retain(|r| r.id != id);
      !inf.writing && inf.racer_li.is_empty()
    } else {
      false
    };

    if empty_and_idle {
      self.in_flight.remove(&idx);
      if self
        .tried_map
        .get(&idx)
        .is_some_and(|t| t.all_set(self.n_src))
      {
        self.err = Some(Error::Exhausted { idx });
        self.abort_all();
      } else {
        self.retry_q.push_back(idx);
      }
      self.wake_all();
    }
  }

  fn remove_racer_only(&mut self, idx: usize, id: u64) {
    if let Some(inf) = self.in_flight.get_mut(&idx) {
      inf.racer_li.retain(|r| r.id != id);
    }
  }

  fn abort_all(&mut self) {
    for inf in self.in_flight.values() {
      for r in &inf.racer_li {
        r.cancel.abort();
      }
    }
  }

  fn poll_ckpt(&mut self, buf: &mut Vec<u8>, cx: &mut Context<'_>) -> Poll<Option<usize>> {
    if self.stop_ckpt || self.err.is_some() || self.done_cnt == self.total_chunk {
      return Poll::Ready(None);
    }
    if !self.flushing && self.dirty_lo < self.dirty_hi {
      let off = self.dirty_lo;
      let end = self.dirty_hi;
      self.dirty_lo = usize::MAX;
      self.dirty_hi = 0;
      self.flushing = true;
      buf.clear();
      buf.extend_from_slice(&self.done_bit_li[off..end]);
      return Poll::Ready(Some(off));
    }
    self.ckpt_waker = Some(cx.waker().clone());
    Poll::Pending
  }

  fn wake_all(&mut self) {
    for slot in &mut self.waker_li {
      if let Some(w) = slot.take() {
        w.wake();
      }
    }
  }

  #[inline]
  fn has_failed(&self, idx: usize, src: usize) -> bool {
    self.tried_map.get(&idx).is_some_and(|t| t.has(src))
  }

  fn spawn_racer(&mut self, idx: usize, src: usize) -> Task {
    self.next_id += 1;
    let id = self.next_id;
    let cancel = Cancel::new();
    let entry = self.in_flight.entry(idx).or_insert_with(|| InFlight {
      writing: false,
      racer_li: Vec::with_capacity(2),
    });
    entry.racer_li.push(Racer {
      id,
      src,
      cancel: cancel.clone(),
    });
    let conn = self.conn_li[src].pop();
    Task::Job {
      idx,
      id,
      src,
      conn,
      cancel,
    }
  }

  fn pick_retry_src(&self, idx: usize, start: usize) -> Option<usize> {
    let mut best: Option<(usize, (u32, usize))> = None;
    for off in 0..self.n_src {
      let s = (start + off) % self.n_src;
      if self.has_failed(idx, s) {
        continue;
      }
      let key = (self.fail_li[s], usize::from(self.conn_li[s].is_empty()));
      if best.is_none_or(|(_, bk)| key < bk) {
        best = Some((s, key));
      }
    }
    best.map(|(s, _)| s)
  }

  fn pick_seg_src(&self, worker: usize, pref: Option<usize>) -> usize {
    if let Some(p) = pref
      && self.fail_li[p] == 0
    {
      return p;
    }
    let start = worker % self.n_src;
    let mut best = start;
    let mut best_key = (
      self.fail_li[start],
      usize::from(self.seg_li[start].len() == 0),
      usize::from(self.conn_li[start].is_empty()),
    );
    for off in 1..self.n_src {
      let s = (start + off) % self.n_src;
      let key = (
        self.fail_li[s],
        usize::from(self.seg_li[s].len() == 0),
        usize::from(self.conn_li[s].is_empty()),
      );
      if key < best_key {
        best = s;
        best_key = key;
      }
    }
    best
  }

  fn take_seg_or_steal(&mut self, src: usize) -> Option<usize> {
    if let Some(idx) = self.seg_li[src].pop_front(&self.done_bit_li) {
      return Some(idx);
    }

    let mut victim: Option<(usize, usize)> = None;
    for (s, seg) in self.seg_li.iter_mut().enumerate() {
      seg.trim_done(&self.done_bit_li);
      let rem = seg.len();
      if rem > 0 && victim.is_none_or(|(_, vr)| rem > vr) {
        victim = Some((s, rem));
      }
    }

    let (v, rem) = victim?;
    if v != src && rem >= 2 {
      // 二分窃取：把最慢源剩余区间的后半段划归当前空闲源
      let mid = self.seg_li[v].cur + rem.div_ceil(2);
      let tail_end = self.seg_li[v].end;
      self.seg_li[v].end = mid;
      self.seg_li[src] = Seg {
        cur: mid,
        end: tail_end,
      };
      if let Some(idx) = self.seg_li[src].pop_front(&self.done_bit_li) {
        return Some(idx);
      }
    }
    self.seg_li[v].pop_front(&self.done_bit_li)
  }

  fn pick_endgame(&self, worker: usize, pref: Option<usize>) -> Option<(usize, usize)> {
    let start = pref.unwrap_or(worker % self.n_src);
    let mut best: Option<((usize, usize), usize, usize)> = None;

    for (&idx, inf) in &self.in_flight {
      if inf.writing || inf.racer_li.len() >= MAX_RACER || bit_get(&self.done_bit_li, idx) {
        continue;
      }
      let tried = self.tried_map.get(&idx);
      let mut cand_src: Option<(usize, (u32, usize, usize))> = None;

      for off in 0..self.n_src {
        let s = (start + off) % self.n_src;
        if tried.is_some_and(|t| t.has(s)) || inf.has_src(s) {
          continue;
        }
        let pref_miss = usize::from(pref != Some(s));
        let key = (
          self.fail_li[s],
          pref_miss,
          usize::from(self.conn_li[s].is_empty()),
        );
        if cand_src.is_none_or(|(_, ck)| key < ck) {
          cand_src = Some((s, key));
        }
      }

      if let Some((src, _)) = cand_src {
        let rank = (inf.racer_li.len(), idx);
        if best.is_none_or(|(br, ..)| rank < br) {
          best = Some((rank, idx, src));
        }
      }
    }

    best.map(|(_, idx, src)| (idx, src))
  }

  fn take_job(&mut self, worker: usize, pref: Option<usize>) -> Task {
    if self.ckpt_aborted || self.err.is_some() || self.done_cnt == self.total_chunk {
      return Task::Done;
    }

    // 1. 优先用偏好健康源消化待重试失败块
    if let Some(p) = pref
      && self.fail_li[p] == 0
      && let Some(pos) = self
        .retry_q
        .iter()
        .position(|&idx| !self.has_failed(idx, p))
      && let Some(idx) = self.retry_q.remove(pos)
    {
      return self.spawn_racer(idx, p);
    }

    // 2. 否则取出队首失败块，切换至尚未尝试过且状态最佳的源
    if let Some(idx) = self.retry_q.pop_front() {
      let start = pref.map_or(worker % self.n_src, |p| (p + 1) % self.n_src);
      let Some(src) = self.pick_retry_src(idx, start) else {
        self.err = Some(Error::Exhausted { idx });
        self.abort_all();
        self.wake_all();
        return Task::Done;
      };
      return self.spawn_racer(idx, src);
    }

    // 3. 消费自有连续分片或二分窃取最慢源的剩余未开始分片
    let src = self.pick_seg_src(worker, pref);
    if let Some(idx) = self.take_seg_or_steal(src) {
      return self.spawn_racer(idx, src);
    }

    // 4. 尾盘竞速：所有未开始分片已耗尽，加入尚未失败且未占用的镜像共同竞速在途块
    if let Some((idx, race_src)) = self.pick_endgame(worker, pref) {
      return self.spawn_racer(idx, race_src);
    }

    Task::Wait
  }
}

/// 挂起空闲工作协程等待新重试任务、尾盘状态变化或整体完成
pub(crate) struct NextTask<'a> {
  pub(crate) sched: &'a RefCell<Sched>,
  pub(crate) worker: usize,
  pub(crate) pref: Option<usize>,
}

impl Future for NextTask<'_> {
  type Output = Task;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut sched = self.sched.borrow_mut();
    match sched.take_job(self.worker, self.pref) {
      Task::Wait => {
        sched.waker_li[self.worker] = Some(cx.waker().clone());
        Poll::Pending
      }
      task => Poll::Ready(task),
    }
  }
}

/// 挂起位图刷盘协程等待脏字节写入
pub(crate) struct NextCkpt<'a> {
  pub(crate) sched: &'a RefCell<Sched>,
  pub(crate) buf: &'a mut Vec<u8>,
}

impl Future for NextCkpt<'_> {
  type Output = Option<usize>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    this.sched.borrow_mut().poll_ckpt(this.buf, cx)
  }
}
