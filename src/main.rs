use core::{cell::RefCell, ffi::c_void, ptr::NonNull};
use elf_loader::{
    Error, Loader,
    mmap::{MapFlags, Mmap, MmapImpl, ProtFlags},
    object::ElfFile,
    segment::PAGE_SIZE,
};
use std::{collections::BTreeMap, env, ffi::CString, process};

struct ProcessRegion {
    base_addr: usize,
    capacity: usize,
    free_blocks: BTreeMap<usize, usize>,
}

impl ProcessRegion {
    fn new(capacity: usize) -> Self {
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                capacity,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if base == libc::MAP_FAILED {
            panic!("failed to allocate process region");
        }

        let base_addr = base as usize;
        let mut free_blocks = BTreeMap::new();
        free_blocks.insert(base_addr, capacity);

        ProcessRegion {
            base_addr,
            capacity,
            free_blocks,
        }
    }

    fn allocate(&mut self, size: usize) -> Option<usize> {
        assert!(size % PAGE_SIZE == 0);

        let target_block = self
            .free_blocks
            .iter()
            .find(|&(_, &block_size)| block_size >= size)
            .map(|(&start, &len)| (start, len));

        if let Some((block_start, block_size)) = target_block {
            self.free_blocks.remove(&block_start);

            let alloc_start = block_start;
            let alloc_end = block_start + size;
            let remaining = block_size - size;

            if remaining > 0 {
                self.free_blocks.insert(alloc_end, remaining);
            }

            Some(alloc_start)
        } else {
            None
        }
    }

    fn deallocate(&mut self, addr: usize, size: usize) {
        assert!(addr >= self.base_addr && addr + size <= self.base_addr + self.capacity);
        assert!(size % PAGE_SIZE == 0);

        let mut new_start = addr;
        let mut new_size = size;

        if let Some((&prev_start, &prev_size)) = self.free_blocks.range(..addr).next_back() {
            if prev_start + prev_size == addr {
                new_start = prev_start;
                new_size += prev_size;
                self.free_blocks.remove(&prev_start);
            }
        }

        if let Some((&next_start, &next_size)) = self.free_blocks.range(addr..).next() {
            if addr + size == next_start {
                new_size += next_size;
                self.free_blocks.remove(&next_start);
            }
        }

        self.free_blocks.insert(new_start, new_size);
    }
}

thread_local! {
    static CURRENT_REGION: RefCell<Option<ProcessRegion>> = RefCell::new(None);
}

fn init_region(capacity: usize) {
    CURRENT_REGION.with(|ctx| {
        *ctx.borrow_mut() = Some(ProcessRegion::new(capacity));
    });
}

struct ProcessLoader;

impl ProcessLoader {
    #[cold]
    #[inline(never)]
    fn map_error(msg: &str) -> Error {
        Error::MmapError {
            msg: msg.to_string(),
        }
    }
}

impl Mmap for ProcessLoader {
    unsafe fn mmap(
        _addr: Option<usize>,
        len: usize,
        prot: ProtFlags,
        _flags: MapFlags,
        offset: usize,
        fd: Option<isize>,
        need_copy: &mut bool,
    ) -> elf_loader::Result<NonNull<c_void>> {
        let addr = CURRENT_REGION.with(|ctx| {
            let mut borrow = ctx.borrow_mut();
            let region = borrow
                .as_mut()
                .ok_or(Self::map_error("no active process region"))?;

            region
                .allocate(len)
                .ok_or(Self::map_error("out of memory in process region"))
        })?;

        let ptr = unsafe { NonNull::new_unchecked(addr as *mut c_void) };
        unsafe { Self::mprotect(ptr, len, ProtFlags::PROT_READ | ProtFlags::PROT_WRITE)? };

        if let Some(fd) = fd {
            let res = unsafe { libc::lseek(fd as i32, offset as i64, libc::SEEK_SET) };
            if res == -1 {
                return Err(Self::map_error("lseek failed"));
            }

            let dst = ptr.as_ptr() as *mut u8;
            let mut total = 0;
            while total < len {
                let remaining = len - total;
                let buf = unsafe { dst.add(total) };
                let n = unsafe { libc::read(fd as i32, buf as *mut c_void, remaining) };
                if n < 0 {
                    return Err(Self::map_error("read failed"));
                }
                if n == 0 {
                    break;
                }

                total += n as usize;
            }

            if total < len {
                let zero_start = unsafe { dst.add(total) };
                unsafe { std::ptr::write_bytes(zero_start, 0, len - total) };
            }

            *need_copy = false;
        } else {
            *need_copy = false;
            unsafe { std::ptr::write_bytes(ptr.as_ptr(), 0, len) };

        }

        println!("allocated {:p} with prot {} and fd {:?}", ptr, prot.bits(), fd);

        unsafe { Self::mprotect(ptr, len, prot)? };
        Ok(ptr)
    }

    unsafe fn mmap_anonymous(
        _addr: usize,
        len: usize,
        prot: ProtFlags,
        flags: MapFlags,
    ) -> elf_loader::Result<NonNull<c_void>> {
        let mut copy = false;
        unsafe { Self::mmap(None, len, prot, flags, 0, None, &mut copy) }
    }

    unsafe fn munmap(addr: NonNull<c_void>, len: usize) -> elf_loader::Result<()> {
        unsafe { Self::mprotect(addr, len, ProtFlags::PROT_NONE)? };
        CURRENT_REGION.with(|ctx| {
            if let Some(region) = ctx.borrow_mut().as_mut() {
                region.deallocate(addr.as_ptr() as usize, len);
                Ok(())
            } else {
                Err(Self::map_error("No region set during munmap"))
            }
        })
    }

    unsafe fn mprotect(
        addr: NonNull<c_void>,
        len: usize,
        prot: ProtFlags,
    ) -> elf_loader::Result<()> {
        let ret = unsafe { libc::mprotect(addr.as_ptr(), len, prot.bits()) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Self::map_error("mprotect failed"))
        }
    }
}

fn resolve_host_symbol(name: &str) -> Option<*const ()> {
    let c_name = CString::new(name).ok()?;
    let addr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_name.as_ptr()) };

    if !addr.is_null() {
        println!("resolving symbol: {} -> {:p}", name, addr);
        Some(addr as *const ())
    } else {
        println!("symbol not found: {}", name);
        None
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lib_name>", args[0]);
        process::exit(1);
    }

    let path = "programs/".to_string() + &args[1];
    println!("loading library from path: {}", path);

    let mut loader = Loader::<ProcessLoader>::new();
    init_region(1024 * 1024 * 1024);

    let object = ElfFile::from_path(&path).unwrap();
    let lib = loader
        .load_dylib(object, None)
        .unwrap()
        .easy_relocate(&[], &resolve_host_symbol)
        .unwrap();

    println!("calling entry point...");
    let entry = unsafe { lib.get::<fn() -> ()>("entry").unwrap() };
    entry();
}
