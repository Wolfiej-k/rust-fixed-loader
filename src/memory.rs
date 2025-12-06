use crate::process::ProcessBounds;
use crate::runtime::set_process_info;
use crate::{LoaderError, LoaderResult};
use elf_loader::mmap::{MapFlags, Mmap, ProtFlags};
use elf_loader::{Error, Result};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;

/// Allowed memory range for current loading operation.
#[derive(Clone, Copy)]
struct MemoryRegion {
    base: usize,
    limit: usize,
    top: usize,
}

thread_local! {
    static ACTIVE_REGION: RefCell<Option<MemoryRegion>> = RefCell::new(None);
}

/// Set active memory region.
fn set_active_region(bounds: Option<ProcessBounds>) {
    ACTIVE_REGION.with(|r| {
        if let Some(b) = bounds {
            *r.borrow_mut() = Some(MemoryRegion {
                base: b.base,
                limit: b.limit,
                top: b.base,
            });
        } else {
            *r.borrow_mut() = None;
        }
    });
}

/// Execute closure with specific active region, resetting afterwards.
pub fn with_region_context<F, R>(bounds: ProcessBounds, f: F) -> R
where
    F: FnOnce() -> R,
{
    set_process_info(Some(bounds));
    set_active_region(Some(bounds));

    let result = f();

    set_process_info(None);
    set_active_region(None);

    result
}

/// mmap-family functions restricted to active region.
pub struct RegionMmap;

impl RegionMmap {
    /// Wrapper for generic anonymous mmap.
    pub fn mmap_next(
        addr: usize,
        len: usize,
        prot: ProtFlags,
    ) -> LoaderResult<NonNull<c_void>, LoaderError> {
        ACTIVE_REGION.with(|reg| {
            let mut region_guard = reg.borrow_mut();
            let region = region_guard
                .as_mut()
                .ok_or_else(|| Self::map_error("No active region context"))?;
            assert!(addr == region.top);
            assert!(addr + len <= region.limit);

            let ptr = unsafe {
                libc::mmap(
                    addr as *mut c_void,
                    len,
                    prot.bits(),
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
                    -1,
                    0,
                )
            };

            if ptr == libc::MAP_FAILED {
                return Err(LoaderError::Mmap("System mmap failed".into()));
            }

            region.top += len;

            Ok(Self::as_nonnull(ptr))
        })
    }

    /// Helper to convert raw pointer to NonNull.
    fn as_nonnull<T>(ptr: *mut T) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(ptr) }
    }

    /// Helper for map failure.
    fn map_error(msg: &str) -> Error {
        Error::MmapError {
            msg: msg.to_string(),
        }
    }
}

impl Mmap for RegionMmap {
    unsafe fn mmap(
        addr: Option<usize>,
        len: usize,
        prot: ProtFlags,
        flags: MapFlags,
        offset: usize,
        fd: Option<isize>,
        need_copy: &mut bool,
    ) -> Result<NonNull<c_void>> {
        ACTIVE_REGION.with(|reg| {
            let mut region_guard = reg.borrow_mut();
            let region = region_guard
                .as_mut()
                .ok_or_else(|| Self::map_error("No active region context"))?;

            // Determine target address
            let map_addr = match addr {
                Some(a) => {
                    // Explicit addresses are allowed under two conditions:
                    //   (1) they are within the region bounds
                    //   (2) MAP_FIXED is set, so we can't ignore it
                    //
                    // This is to maintain compatibility with elf_loader
                    if a < region.base || a >= region.limit {
                        return Err(Self::map_error("Requested address out of bounds"));
                    }
                    if !flags.contains(MapFlags::MAP_FIXED) {
                        return Err(Self::map_error("Explicit address requires MAP_FIXED"));
                    }
                    a
                }
                None => region.top,
            };

            // Check region capacity
            if map_addr + len > region.limit {
                return Err(Self::map_error("Out of region memory"));
            }

            // Weird elf_loader semantics: since `reserve` is called before
            // `mmap`, we only perform a mapping if a fd is present
            let ptr = if let Some(fd) = fd {
                unsafe {
                    libc::mmap(
                        map_addr as *mut c_void,
                        len,
                        prot.bits(),
                        flags.union(MapFlags::MAP_FIXED).bits(),
                        fd as i32,
                        offset as i64,
                    )
                }
            } else {
                *need_copy = true;
                map_addr as *mut c_void
            };

            // We expect mmap to return the address we asked for
            if ptr == libc::MAP_FAILED || (fd.is_some() && ptr as usize != map_addr) {
                return Err(Self::map_error("System mmap failed"));
            }

            // Extend the region top if needed
            if addr.is_none() {
                region.top += len;
            }

            Ok(Self::as_nonnull(ptr))
        })
    }

    unsafe fn mmap_anonymous(
        addr: usize,
        len: usize,
        prot: ProtFlags,
        flags: MapFlags,
    ) -> Result<NonNull<c_void>> {
        // Always called with a fixed address; again, elf_loader is weird
        ACTIVE_REGION.with(|reg| {
            let guard = reg.borrow();
            if let Some(region) = &*guard {
                if addr < region.base || addr + len > region.limit {
                    return Err(Self::map_error("Requested address out of bounds"));
                }
                Ok(())
            } else {
                Err(Self::map_error("No active region context"))
            }
        })?;

        let ptr = unsafe {
            libc::mmap(
                addr as _,
                len,
                prot.bits(),
                flags
                    .union(MapFlags::MAP_FIXED | MapFlags::MAP_ANONYMOUS)
                    .bits(),
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED || ptr as usize != addr {
            return Err(Self::map_error("System mmap failed"));
        }

        Ok(Self::as_nonnull(ptr))
    }

    unsafe fn munmap(addr: NonNull<c_void>, len: usize) -> elf_loader::Result<()> {
        if unsafe { libc::munmap(addr.as_ptr(), len) } != 0 {
            return Err(Self::map_error("System munmap failed"));
        }
        Ok(())
    }

    unsafe fn mprotect(addr: NonNull<c_void>, len: usize, prot: ProtFlags) -> Result<()> {
        if unsafe { libc::mprotect(addr.as_ptr(), len, prot.bits()) } != 0 {
            return Err(Self::map_error("System mprotect failed"));
        }
        Ok(())
    }

    unsafe fn mmap_reserve(
        _addr: Option<usize>,
        len: usize,
        use_file: bool,
    ) -> Result<NonNull<c_void>> {
        ACTIVE_REGION.with(|reg| {
            let mut region_guard = reg.borrow_mut();
            let region = region_guard
                .as_mut()
                .ok_or_else(|| Self::map_error("No active region context"))?;

            // If `use_file` is set, `mmap` will eventually be called on this
            // region with a file descriptor, so we just use PROT_NONE
            let flags = MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS | MapFlags::MAP_FIXED;
            let prot = if use_file {
                ProtFlags::PROT_NONE
            } else {
                ProtFlags::PROT_WRITE
            };

            // Now this is a typical bump allocation
            let map_addr = region.top;
            if map_addr + len > region.limit {
                return Err(Self::map_error("Out of region memory"));
            }

            let ptr = unsafe { libc::mmap(map_addr as _, len, prot.bits(), flags.bits(), -1, 0) };
            if ptr == libc::MAP_FAILED || ptr as usize != map_addr {
                return Err(Self::map_error("mmap_reserve failed"));
            }

            region.top += len;
            Ok(Self::as_nonnull(ptr))
        })
    }
}
