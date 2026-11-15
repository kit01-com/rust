use std::{
  ffi::OsString,
  path::{Path, PathBuf},
};

use compio::{
  BufResult,
  fs::{File, rename},
  io::{AsyncReadAtExt, AsyncWriteAtExt},
};

use crate::{
  error::{Error, Result},
  sched::plan_chunk_size,
};

/// 断点续传临时文件后缀
pub const PART_EXT: &str = ".part";

const MAGIC: [u8; 8] = *b"MSRCGET1";
const FOOTER_LEN: usize = 24;

#[cfg(unix)]
mod sys {
  use std::{
    fs::{File as StdFile, TryLockError},
    os::fd::AsFd,
    path::Path,
  };

  use compio::fs::{File, OpenOptions};

  use crate::error::{Error, Result};

  pub(super) struct Guard(StdFile);

  pub(super) async fn open_part(part: &Path) -> Result<(File, Guard)> {
    let file = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(false)
      .open(part)
      .await?;
    let lock = StdFile::from(file.as_fd().try_clone_to_owned()?);
    match lock.try_lock() {
      Ok(()) => Ok((file, Guard(lock))),
      Err(TryLockError::WouldBlock) => Err(Error::Locked),
      Err(TryLockError::Error(err)) => Err(err.into()),
    }
  }

  #[inline]
  pub(super) fn set_sparse(_file: &File) {}

  pub(super) fn release(guard: Guard) {
    let _ = guard.0.unlock();
  }
}

#[cfg(windows)]
mod sys {
  use std::{
    ffi::c_void,
    os::windows::io::{AsRawHandle, RawHandle},
    path::Path,
    ptr::null_mut,
  };

  use compio::fs::{File, OpenOptions};

  use crate::error::{Error, Result};

  const FSCTL_SET_SPARSE: u32 = 0x0009_00C4;
  const FILE_SHARE_READ: u32 = 1;
  const FILE_SHARE_DELETE: u32 = 4;
  const ERROR_SHARING_VIOLATION: i32 = 32;

  #[link(name = "kernel32")]
  unsafe extern "system" {
    fn DeviceIoControl(
      h_device: RawHandle,
      dw_io_control_code: u32,
      lp_in_buffer: *const c_void,
      n_in_buffer_size: u32,
      lp_out_buffer: *mut c_void,
      n_out_buffer_size: u32,
      lp_bytes_returned: *mut u32,
      lp_overlapped: *mut c_void,
    ) -> i32;
  }

  pub(super) struct Guard;

  pub(super) async fn open_part(part: &Path) -> Result<(File, Guard)> {
    let res = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(false)
      .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
      .open(part)
      .await;
    match res {
      Ok(file) => Ok((file, Guard)),
      Err(err) if err.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Err(Error::Locked),
      Err(err) => Err(err.into()),
    }
  }

  pub(super) fn set_sparse(file: &File) {
    let mut returned: u32 = 0;
    let _ = unsafe {
      DeviceIoControl(
        file.as_raw_handle(),
        FSCTL_SET_SPARSE,
        null_mut(),
        0,
        null_mut(),
        0,
        &mut returned,
        null_mut(),
      )
    };
  }

  #[inline]
  pub(super) fn release(_guard: Guard) {}
}

/// 生成目标文件对应的临时续传路径（在文件名末尾追加 `.part`）
pub fn part_path(path: impl AsRef<Path>) -> PathBuf {
  let mut raw: OsString = path.as_ref().as_os_str().to_os_string();
  raw.push(PART_EXT);
  PathBuf::from(raw)
}

#[inline]
pub(crate) fn bit_get(bit_li: &[u8], idx: usize) -> bool {
  (bit_li[idx >> 3] >> (idx & 7)) & 1 != 0
}

#[inline]
pub(crate) fn bit_set(bit_li: &mut [u8], idx: usize) {
  bit_li[idx >> 3] |= 1u8 << (idx & 7);
}

struct Layout {
  total_chunk: usize,
  bitmap_len: usize,
  phys_len: u64,
}

fn calc_layout(total_size: u64, chunk_size: usize) -> Result<Layout> {
  if chunk_size == 0 {
    return Err(Error::ChunkSize);
  }
  let total_chunk =
    usize::try_from(total_size.div_ceil(chunk_size as u64)).map_err(|_| Error::ChunkSize)?;
  let bitmap_len = total_chunk.div_ceil(8);
  let trailer_len = (bitmap_len as u64)
    .checked_add(FOOTER_LEN as u64)
    .ok_or(Error::ChunkSize)?;
  let phys_len = total_size
    .checked_add(trailer_len)
    .ok_or(Error::ChunkSize)?;
  Ok(Layout {
    total_chunk,
    bitmap_len,
    phys_len,
  })
}

fn encode_footer(total_size: u64, chunk_size: usize) -> [u8; FOOTER_LEN] {
  let mut foot = [0u8; FOOTER_LEN];
  foot[..8].copy_from_slice(&total_size.to_le_bytes());
  foot[8..16].copy_from_slice(&(chunk_size as u64).to_le_bytes());
  foot[16..24].copy_from_slice(&MAGIC);
  foot
}

fn decode_footer(foot: &[u8; FOOTER_LEN]) -> Option<(u64, usize)> {
  let (&size_b, rest) = foot.split_first_chunk::<8>()?;
  let (&chunk_b, magic) = rest.split_first_chunk::<8>()?;
  if magic != MAGIC {
    return None;
  }
  let file_size = u64::from_le_bytes(size_b);
  let chunk_size = usize::try_from(u64::from_le_bytes(chunk_b)).ok()?;
  if chunk_size == 0 {
    return None;
  }
  Some((file_size, chunk_size))
}

/// 已打开且加锁的稀疏临时下载文件（含尾部完成位图）
pub(crate) struct PartFile {
  pub(crate) file: File,
  guard: sys::Guard,
  target: PathBuf,
  part: PathBuf,
  total_size: u64,
}

/// 打开或新建稀疏临时文件后的初始恢复状态
pub(crate) struct OpenState {
  pub(crate) part: PartFile,
  pub(crate) chunk_size: usize,
  pub(crate) total_chunk: usize,
  pub(crate) done_bit_li: Vec<u8>,
  pub(crate) done_cnt: usize,
}

impl PartFile {
  /// 处理 0 字节文件：创建并加锁 `.part` 截断为 0 后原子重命名为目标路径
  pub(crate) async fn finish_empty(target: &Path) -> Result<()> {
    let part = part_path(target);
    let (file, guard) = sys::open_part(&part).await?;
    file.set_len(0).await?;
    file.sync_data().await?;
    file.close().await?;
    // 先重命名再解锁，防止竞争进程在空隙误开并覆写刚完成的文件
    let res = rename(&part, target).await;
    sys::release(guard);
    res?;
    Ok(())
  }

  /// 打开（或新建稀疏）`.part` 文件并尝试从尾部元数据恢复断点进度
  pub(crate) async fn open(
    target: &Path,
    total_size: u64,
    user_chunk: Option<usize>,
    conc: usize,
  ) -> Result<OpenState> {
    let part = part_path(target);
    let (file, guard) = sys::open_part(&part).await?;
    let cur_len = file.metadata().await?.len();

    if cur_len >= FOOTER_LEN as u64 {
      let foot_buf = [0u8; FOOTER_LEN];
      let BufResult(res, foot) = file
        .read_exact_at(foot_buf, cur_len - FOOTER_LEN as u64)
        .await;
      if res.is_ok()
        && let Some((saved_size, saved_chunk)) = decode_footer(&foot)
        && saved_size == total_size
        && user_chunk.is_none_or(|c| c == saved_chunk)
        && let Ok(layout) = calc_layout(total_size, saved_chunk)
        && cur_len == layout.phys_len
      {
        sys::set_sparse(&file);
        let bit_buf = vec![0u8; layout.bitmap_len];
        let BufResult(b_res, mut done_bit_li) = file.read_exact_at(bit_buf, total_size).await;
        if b_res.is_ok() {
          let rem = layout.total_chunk & 7;
          if rem != 0
            && let Some(last) = done_bit_li.last_mut()
          {
            *last &= (1u8 << rem) - 1;
          }
          let done_cnt: usize = done_bit_li.iter().map(|b| b.count_ones() as usize).sum();
          return Ok(OpenState {
            part: Self {
              file,
              guard,
              target: target.to_path_buf(),
              part,
              total_size,
            },
            chunk_size: saved_chunk,
            total_chunk: layout.total_chunk,
            done_bit_li,
            done_cnt,
          });
        }
      }
    }

    let chunk_size = user_chunk.unwrap_or_else(|| plan_chunk_size(total_size, conc));
    let layout = calc_layout(total_size, chunk_size)?;

    // 旧 `.part` 无有效断点数据时归零重建稀疏洞，保证尾部初始位图全 0
    if cur_len > 0 {
      file.set_len(0).await?;
    }
    sys::set_sparse(&file);
    file.set_len(layout.phys_len).await?;

    let foot = encode_footer(total_size, chunk_size);
    let mut f = &file;
    let BufResult(w_res, _) = f
      .write_all_at(foot, layout.phys_len - FOOTER_LEN as u64)
      .await;
    w_res?;
    file.sync_data().await?;

    Ok(OpenState {
      part: Self {
        file,
        guard,
        target: target.to_path_buf(),
        part,
        total_size,
      },
      chunk_size,
      total_chunk: layout.total_chunk,
      done_bit_li: vec![0u8; layout.bitmap_len],
      done_cnt: 0,
    })
  }

  /// 将脏位图切片原地写入末尾位图对应偏移并归还缓冲
  pub(crate) async fn write_bitmap(&self, off: usize, buf: Vec<u8>) -> Result<Vec<u8>> {
    let mut f = &self.file;
    let BufResult(res, back) = f.write_all_at(buf, self.total_size + off as u64).await;
    res?;
    Ok(back)
  }

  /// 下载中断时尽力落盘位图并关闭句柄释放锁（保留 `.part` 供下次续传）
  pub(crate) async fn abort_keep(self, dirty: Option<(usize, Vec<u8>)>) {
    if let Some((off, buf)) = dirty {
      let _ = self.write_bitmap(off, buf).await;
    }
    let _ = self.file.sync_data().await;
    let _ = self.file.close().await;
    sys::release(self.guard);
  }

  /// 全部块下载校验通过：截断尾部元数据 -> 刷盘 -> 关闭 -> 重命名 -> 解锁
  pub(crate) async fn finalize(self) -> Result<()> {
    self.file.set_len(self.total_size).await?;
    self.file.sync_data().await?;
    self.file.close().await?;
    // 先重命名再解锁，避免截断后解锁的空隙被外部进程误判为损坏并清零
    let res = rename(&self.part, &self.target).await;
    sys::release(self.guard);
    res?;
    Ok(())
  }
}
