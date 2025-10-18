/// Provides lock-free concurrent writing and reading of frames in a shared memory region.
use core::iter::from_fn;
use std::{
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    ptr::slice_from_raw_parts_mut,
    sync::atomic::{AtomicI32, AtomicUsize, Ordering, fence},
};

use bytemuck::must_cast;
use memmap2::MmapRaw;
use shared_memory::Shmem;

// `ShmWriter` writes headers using atomic operations to prevent partial writes due to crashes,
// while `ShmReader` reads headers by simple pointer dereferences.
// This is safe because `ShmReader` is only used after all writing is done and visible to the calling thread (see docs of `ShmReader::new`).
// To ensure that the layouts of atomic types and their non-atomic counterparts are the same:
const _: () = {
    assert!(size_of::<usize>() == size_of::<AtomicUsize>());
    assert!(align_of::<usize>() == align_of::<AtomicUsize>());
    assert!(size_of::<i32>() == size_of::<AtomicI32>());
    assert!(align_of::<i32>() == align_of::<AtomicI32>());
};

/// A trait to borrow a raw memory region.
pub trait AsRawSlice {
    fn as_raw_slice(&self) -> *mut [u8];
}

impl AsRawSlice for Shmem {
    fn as_raw_slice(&self) -> *mut [u8] {
        slice_from_raw_parts_mut(self.as_ptr(), self.len())
    }
}

impl AsRawSlice for MmapRaw {
    fn as_raw_slice(&self) -> *mut [u8] {
        slice_from_raw_parts_mut(self.as_mut_ptr(), self.len())
    }
}

/// A concurrent shared memory writer.
///
/// It's lock-free and safe to use across multiple threads/processes at the same time.
/// Internally it uses atomic operations to ensure that multiple writers can write to the shared memory without
/// overwriting each other's data.
pub struct ShmWriter<M> {
    /*
    Layout of the whole shared memory:
    | total byte size of frames(AtomicUsize) | frame 1 | frame 2 | ..... |

    Possible layout states of each frame:
    - | 0(AtomicI32) | 0000...... | all zero. This happens when the thread/process crashed right after the frame is claimed.
    - | byte size of the frame (AtomicI32) | partially written data | extra 0s to align to next frame header | This happens when the thread/process crashed during writing.
    - | negative byte size of the frame (AtomicI32) | fully written data | extra 0s to align to next frame header | This is the normal case (negative size indicates completion).
    */
    mem: M,

    #[cfg(test)]
    fail_on_claim: bool,
}

// unsafe impl<M: AsRawMemory> Send for ShmWriter<M> {}
// unsafe impl<M: AsRawMemory> Sync for ShmWriter<M> {}

#[track_caller]
fn assert_alignment(ptr: *const u8) {
    // Assert that the header of the shm is aligned to usize
    assert_eq!(ptr as usize % align_of::<usize>(), 0);
    // Assert that the content after whole shm header is aligned to i32
    assert_eq!((ptr as usize + size_of::<usize>()) % align_of::<i32>(), 0);
}

fn roundup_to_align_frame_header(mut size: usize) -> usize {
    // round up new_end so that the next frame header is aligned
    const FRAME_HEADER_ALIGN: usize = align_of::<AtomicI32>();
    if size % FRAME_HEADER_ALIGN != 0 {
        size += FRAME_HEADER_ALIGN - (size % FRAME_HEADER_ALIGN);
    }
    size
}

pub struct FrameMut<'a> {
    header: &'a AtomicI32,
    content: &'a mut [u8],
}
impl Deref for FrameMut<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.content
    }
}
impl DerefMut for FrameMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.content
    }
}

impl Drop for FrameMut<'_> {
    fn drop(&mut self) {
        // Prevents compiler from ordering memory operations. Ensure the data is visible before marking as fully written
        fence(Ordering::Release);

        // Mark as fully written (negative size indicates completion)
        let frame_size_i32 =
            i32::try_from(self.content.len()).expect("frame size checked in `append_frame`");
        self.header.store(-frame_size_i32, Ordering::Relaxed);
    }
}

impl<M: AsRawSlice> ShmWriter<M> {
    /// Create a new `ShmCursor` backed by a shared memory region.
    ///
    /// # Safety
    /// - `mem.as_raw_slice()` must return a stable valid pointer to a memory region of `total` bytes,
    /// - the memory region must only be accessed via `ShmCursor` across all the processes.
    /// - The unused region of the shared memory must be initialized to zero.
    pub unsafe fn new(mem: M) -> Self {
        assert_alignment(mem.as_raw_slice() as *const u8);
        Self {
            mem,
            #[cfg(test)]
            fail_on_claim: false,
        }
    }

    // Unwrap `self` and return the underlying memory.
    pub fn into_memory(self) -> M {
        self.mem
    }

    #[cfg(test)]
    fn set_fail_on_claim(&mut self, fail_on_claim: bool) {
        self.fail_on_claim = fail_on_claim;
    }

    /// Claim a frame of size `frame_size`.
    ///
    /// Returns `None` if there is no sufficient remaining space (or simulated crash in tests)
    /// frame_size must be non-zero because frame header being 0 would be ambiguous.
    pub fn claim_frame(&self, frame_size: NonZeroUsize) -> Option<FrameMut<'_>> {
        let shm_slice: *mut [u8] = self.mem.as_raw_slice();
        let shm_ptr = shm_slice as *mut u8;
        let shm_len = self.mem.as_raw_slice().len();

        let frame_size = frame_size.get();
        let Ok(frame_size_i32) = i32::try_from(frame_size) else {
            // negative frame size is meant to indicate completion. So we can't write frame larger than i32::MAX.
            return None;
        };

        // Get the atomic value of the end position (first 8 bytes of shared memory)
        let atomic_header = unsafe { AtomicUsize::from_ptr(shm_ptr.cast()) };

        let frame_with_header_size = size_of::<i32>() + frame_size;

        // Try to atomically claim the space
        // Different writers only share the header, not each other's content. so relaxed ordering is sufficient.
        let current_end =
            atomic_header.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_end| {
                let new_end = roundup_to_align_frame_header(current_end + frame_with_header_size);

                // Check if we have enough space
                if size_of::<usize>() + new_end > shm_len {
                    return None;
                }

                Some(new_end)
            });

        let Ok(current_end) = current_end else {
            return None; // Not enough space
        };

        #[cfg(test)]
        if self.fail_on_claim {
            // Simulate crash right after claiming the space
            return None;
        }

        // Successfully claimed the space, now write the data

        let frame_start = unsafe {
            shm_ptr.add(/* shm header */ size_of::<usize>() + current_end)
        };

        let frame_header = unsafe { AtomicI32::from_ptr(frame_start.cast()) };

        // Mark as partially written with positive size
        // Atomic operations on the frame header is only for preventing partial writes of the frame header itself (possibly due to crashes),
        // not for synchronization of frame contents, so relaxed ordering is sufficient
        frame_header.store(frame_size_i32, Ordering::Relaxed);

        // Prevents compiler from re-ordering memory operations. Ensure the size is visible before writing the data
        fence(Ordering::Release);

        let frame_content_ptr = unsafe { frame_start.add(size_of::<i32>()) }; // skip the frame header
        Some(FrameMut {
            header: frame_header,
            content: unsafe { std::slice::from_raw_parts_mut(frame_content_ptr, frame_size) },
        })
    }

    pub fn append_frame(&self, frame: &[u8]) -> bool {
        let Some(frame_size) = NonZeroUsize::new(frame.len()) else {
            return false;
        };
        let Some(mut frame_mut) = self.claim_frame(frame_size) else {
            return false;
        };
        frame_mut.copy_from_slice(frame);
        true
    }
}

/// Reader of frames in shared memory created by `ShmWriter`.
pub struct ShmReader<M: AsRef<[u8]>> {
    mem: M,
}

impl<M: AsRef<[u8]>> ShmReader<M> {
    /// The content of `mem` should be created by `ShmWriter`.
    /// Failing to do so may result in panics (mostly out-of-bounds), but won't trigger undefined behavior.
    ///
    /// The ShmReader must be created after all writing to the shared memory is done and visible to the calling thread.
    /// This is guaranteed by `M: AsRef<[u8]>`, which means the memory region is immutable during the lifetime of `ShmReader`,
    /// so no need to mark `ShmReader::new` as unsafe, but care must be taken to create a safe `M` from the shared memory.
    pub fn new(mem: M) -> Self {
        assert_alignment(mem.as_ref().as_ptr());
        Self { mem }
    }

    /// Iterate over all the frames in the shared memory.
    pub fn iter_frames(&self) -> impl Iterator<Item = &[u8]> {
        let mem = self.mem.as_ref();
        let (header, content) = mem
            .split_first_chunk::<{ size_of::<usize>() }>()
            .expect("mem too small to contain header");
        let content_size: usize = must_cast(*header);
        let mut remaining_content = &content[..content_size];

        from_fn(move || {
            let frame_size = loop {
                // looking for the next valid frame
                let (frame_header, next_remaining_content) =
                    remaining_content.split_first_chunk::<{ size_of::<i32>() }>()?;
                remaining_content = next_remaining_content;
                let frame_header: i32 = must_cast(*frame_header);
                match frame_header {
                    0 => {
                        // frame was claimed but never written (crashed process)
                        // Keep reading until we find a non-zero header
                        continue;
                    }
                    1.. => {
                        // Partially written frame - skip it and continue
                        let size = usize::try_from(frame_header).unwrap();
                        remaining_content =
                            &remaining_content[roundup_to_align_frame_header(size)..];
                        continue;
                    }
                    ..0 => {
                        // Fully written frame (negative size indicates completion)
                        break usize::try_from(-frame_header).unwrap();
                    }
                }
            };

            let (frame_with_padding, next_remaining_content) =
                remaining_content.split_at(roundup_to_align_frame_header(frame_size));
            remaining_content = next_remaining_content;

            Some(&frame_with_padding[..frame_size])
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        env::current_exe,
        process::{Child, Command},
        sync::Arc,
        thread,
    };

    use assert2::assert;
    use bstr::BStr;

    use super::*;

    /// A mocked shared memory region for testing.
    ///
    /// To be testable for miri, the shared memory is allocated using `Arc` instead of real shared memory APIs.
    #[derive(Clone)]
    struct MockedShm {
        // Why usize: to ensure alignment
        //
        // Why not Arc<[usize]>:
        // According to miri, from the perspective of data racing, incrementing ref count of Arc<[T]>
        // is considered the same as reading the content of [T], which conflicts with writing to [T] by `ShmWriter`.
        // This problem is unrelated to real shared memory.
        mem: Arc<Vec<usize>>,
        /// The actual requested byte length.
        ///
        /// over-allocation might happen to ensure alignment of `usize`, so mem.len() might be inaccurate.
        len: usize,
    }
    unsafe impl Send for MockedShm {}
    unsafe impl Sync for MockedShm {}
    impl MockedShm {
        fn alloc(len: usize) -> Self {
            // allocates this many of usize to fit the requested byte size
            let size_in_usize = len / size_of::<usize>() + 1;

            let mem: Vec<usize> = core::iter::repeat(0usize).take(size_in_usize).collect();

            Self { mem: Arc::new(mem), len }
        }
    }
    impl AsRef<[u8]> for MockedShm {
        fn as_ref(&self) -> &[u8] {
            unsafe { std::slice::from_raw_parts(Vec::as_ptr(&self.mem).cast(), self.len) }
        }
    }

    impl AsRawSlice for MockedShm {
        fn as_raw_slice(&self) -> *mut [u8] {
            slice_from_raw_parts_mut(Vec::as_ptr(&self.mem).cast::<u8>().cast_mut(), self.len)
        }
    }

    #[test]
    fn single_thread_basic() {
        let writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };
        assert!(writer.append_frame(b"hello"));
        assert!(writer.append_frame(b"world"));
        assert!(writer.append_frame(b"this is a test"));
        assert!(!writer.append_frame(&vec![0u8; 2048])); // too large

        let reader = ShmReader::new(writer.into_memory());
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"hello");
        assert_eq!(frames.next().unwrap(), b"world");
        assert_eq!(frames.next().unwrap(), b"this is a test");
        assert_eq!(frames.next(), None);
    }
    #[test]
    fn single_thread_empty() {
        let writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };
        assert!(writer.append_frame(b"hello"));
        assert!(!writer.append_frame(b""));
        assert!(writer.append_frame(b"this is a test"));

        let reader = ShmReader::new(writer.into_memory());
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"hello");
        assert_eq!(frames.next().unwrap(), b"this is a test");
        assert_eq!(frames.next(), None);
    }

    #[test]
    fn single_thread_crash_after_claim() {
        let mut writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };
        assert!(writer.append_frame(b"foo"));

        // Simulate crash during writing
        writer.set_fail_on_claim(true);
        assert!(!writer.append_frame(b"hello"));

        writer.set_fail_on_claim(false);
        assert!(writer.append_frame(b"bar"));

        let reader = ShmReader::new(writer.into_memory());
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"foo");
        assert_eq!(frames.next().unwrap(), b"bar");
        assert_eq!(frames.next(), None);
    }

    #[test]
    fn single_thread_crash_partial_write() {
        let writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };
        assert!(writer.append_frame(b"foo"));

        // Simulate crash during writing
        let mut frame = writer.claim_frame(5.try_into().unwrap()).unwrap();
        frame[..3].copy_from_slice(b"wor");
        std::mem::forget(frame);

        assert!(writer.append_frame(b"bar"));

        let reader = ShmReader::new(writer.into_memory());
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"foo");
        assert_eq!(frames.next().unwrap(), b"bar");
        assert_eq!(frames.next(), None);
    }

    #[test]
    fn single_thread_two_crashes_after_claim_and_partial_write() {
        // This test verifies that ShmReader::iter correctly handles MULTIPLE consecutive
        // invalid frames by continuing the loop. It's crucial for testing
        // that the reader doesn't stop at the first invalid frame but keeps processing
        // through multiple crash scenarios to find valid frames beyond them.

        let mut writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };

        assert!(writer.append_frame(b"foo"));

        // First crash: AfterClaim (leaves frame header as 0)
        writer.set_fail_on_claim(true);
        assert!(!writer.append_frame(b"world"));
        writer.set_fail_on_claim(false);

        // Second crash: PartialWrite (leaves positive frame header)
        let mut frame = writer.claim_frame(5.try_into().unwrap()).unwrap();
        frame[..3].copy_from_slice(b"wor");
        std::mem::forget(frame);

        assert!(writer.append_frame(b"bar"));

        // ShmReader must skip BOTH invalid frames (0 header + partial header)
        // and find the valid frame beyond them - this tests the loop continuation

        let reader = ShmReader::new(writer.into_memory());
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"foo");
        assert_eq!(frames.next().unwrap(), b"bar");
        assert_eq!(frames.next(), None);
    }

    #[test]
    fn single_thread_two_crashes_partial_write_and_after_claim() {
        let mut writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };
        // This test verifies the same loop continuation behavior but with crashes
        // in reverse order. This ensures the loop correctly handles different
        // sequences of invalid frame types (partial write -> after claim).

        assert!(writer.append_frame(b"foo"));

        // First crash: PartialWrite (leaves positive frame header)
        let mut frame = writer.claim_frame(5.try_into().unwrap()).unwrap();
        frame[..3].copy_from_slice(b"wor");
        std::mem::forget(frame);

        // Second crash: AfterClaim (leaves frame header as 0)
        writer.set_fail_on_claim(true);
        assert!(!writer.append_frame(b"world"));
        writer.set_fail_on_claim(false);

        assert!(writer.append_frame(b"bar"));

        let reader = ShmReader::new(writer.into_memory());
        // ShmReader must skip BOTH invalid frames in this order and continue
        // processing to find valid frames - tests loop robustness
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"foo");
        assert_eq!(frames.next().unwrap(), b"bar");
        assert_eq!(frames.next(), None);
    }

    #[test]
    fn concurrent() {
        let shm = MockedShm::alloc(1024 * 4);

        thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| {
                    let writer = unsafe { ShmWriter::new(shm.clone()) };
                    for _ in 0..10 {
                        assert!(writer.append_frame(b"hello"));
                        assert!(writer.append_frame(b"foo"));
                        assert!(writer.append_frame(b"this is a test"));
                    }
                });
            }
        });
        let mut count = 0;
        let reader = ShmReader::new(shm);
        for frame in reader.iter_frames() {
            count += 1;
            let frame = BStr::new(frame);
            assert!(frame == b"hello" || frame == b"foo" || frame == b"this is a test");
        }
        assert_eq!(count, 120);
    }

    #[test]
    fn concurrent_exceeded_size() {
        let writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };
        thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| {
                    for _ in 0..10 {
                        writer.append_frame(b"hello");
                        writer.append_frame(b"foo");
                        writer.append_frame(b"this is a test");
                    }
                });
            }
        });
        let mut count = 0;
        let reader = ShmReader::new(writer.into_memory());
        for frame in reader.iter_frames() {
            count += 1;
            let frame = BStr::new(frame);
            assert!(frame == b"hello" || frame == b"foo" || frame == b"this is a test");
        }
        assert!(count > 50);
    }

    #[test]
    fn test_integer_overflow_space_calculation() {
        // Test case for potential integer overflow in space calculation

        let writer = unsafe { ShmWriter::new(MockedShm::alloc(1024)) };

        // Try to trigger integer overflow by using maximum values
        let large_frame = vec![0u8; (i32::MAX as usize) - 100];

        // This should fail safely, not cause overflow
        assert!(!writer.append_frame(&large_frame));

        // Small frame should still work
        assert!(writer.append_frame(b"test"));

        let reader = ShmReader::new(writer.into_memory());
        let mut frames = reader.iter_frames();
        assert_eq!(frames.next().unwrap(), b"test");
        assert_eq!(frames.next(), None);
    }

    #[test]
    fn test_space_calculation_race_condition() {
        // Test for race condition in space calculation where multiple threads
        // might calculate overlapping space requirements

        let writer = unsafe { ShmWriter::new(MockedShm::alloc(200)) };

        // Very small buffer
        thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    // Many threads trying to write large-ish frames
                    writer.append_frame(b"this_is_a_moderately_long_frame_that_might_cause_races");
                });
            }
        });

        // The exact count doesn't matter, but the reader should not panic
        // and should handle any race conditions gracefully

        let reader = ShmReader::new(writer.into_memory());
        let mut count = 0;
        for _frame in reader.iter_frames() {
            count += 1;
        }
        // At least some but not all writes should succeed
        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_alignment_violation_detection() {
        struct Misaligned(MockedShm);
        impl AsRawSlice for Misaligned {
            fn as_raw_slice(&self) -> *mut [u8] {
                let raw_slice = self.0.as_raw_slice();
                slice_from_raw_parts_mut(
                    unsafe { raw_slice.cast::<u8>().add(1) },
                    raw_slice.len() - 1,
                )
            }
        }
        // Test that alignment violations are properly detected

        // Allocate memory with proper alignment first
        let shm = MockedShm::alloc(64);

        // Create a deliberately misaligned pointer by adding 1 byte
        // This ensures the pointer is NOT aligned to usize boundary
        let misaligned_shm = Misaligned(shm);

        // Verify the pointer is actually misaligned
        assert_ne!(misaligned_shm.as_raw_slice().cast::<u8>() as usize % align_of::<usize>(), 0);

        // This should panic due to alignment assertion
        let result = std::panic::catch_unwind(|| {
            unsafe { ShmWriter::new(misaligned_shm) };
        });

        // Verify that the alignment check properly caught the violation
        assert!(result.is_err(), "Should panic on misaligned pointer");
    }

    #[test]
    #[cfg(not(miri))]
    fn real_shm_across_processes() {
        use shared_memory::ShmemConf;

        const CHILD_PROCESS_ENV: &str = "FSPY_SHM_IO_TEST_CHILD_PROCESS";
        const CHILD_COUNT: usize = 12;
        const FRAME_COUNT_EACH_CHILD: usize = 100;

        #[ctor::ctor]
        fn child_process() {
            if std::env::var_os(CHILD_PROCESS_ENV).is_none() {
                return;
            }
            let mut args = std::env::args_os();
            args.next().unwrap(); // exe path
            let shm_name = args.next().expect("shm name arg").into_string().unwrap();
            let child_index = args.next().expect("child name").into_string().unwrap();

            let shm = ShmemConf::new().os_id(shm_name).open().unwrap();
            let writer = unsafe { ShmWriter::new(shm) };
            for i in 0..FRAME_COUNT_EACH_CHILD {
                assert!(writer.append_frame(format!("{child_index} {i}").as_bytes()));
            }
            std::process::exit(0);
        }

        let shm = ShmemConf::new().size(1024 * 1024).create().unwrap();
        let shm_name = shm.get_os_id();

        let children: Vec<Child> = (0..CHILD_COUNT)
            .map(|child_index| {
                Command::new(current_exe().unwrap())
                    .env(CHILD_PROCESS_ENV, "1")
                    .arg(shm_name)
                    .arg(child_index.to_string())
                    .spawn()
                    .unwrap()
            })
            .collect();

        for mut c in children {
            let status = c.wait().unwrap();
            assert!(status.success());
        }

        let shm = unsafe { shm.as_slice() };
        let reader = ShmReader::new(shm);
        let frames = reader.iter_frames().map(BStr::new).collect::<HashSet<&BStr>>();
        assert_eq!(frames.len(), CHILD_COUNT * FRAME_COUNT_EACH_CHILD);
        for child_index in 0..CHILD_COUNT {
            for i in 0..FRAME_COUNT_EACH_CHILD {
                assert!(frames.contains(&BStr::new(format!("{child_index} {i}").as_bytes())));
            }
        }
    }
}
