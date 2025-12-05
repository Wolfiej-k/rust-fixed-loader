use core::{ffi::c_void, ptr::NonNull};
use elf_loader::{
    Error, Loader,
    mmap::{MapFlags, Mmap, ProtFlags},
    object::ElfFile,
};
use std::{env, ffi::CString, process};

static PROCESS_SIZE: usize = 1024 * 1024 * 1024;
static mut PROCESS_BASE: Option<usize> = None;
static mut PROCESS_USED: usize = 0;

fn process_base() -> usize {
    unsafe { PROCESS_BASE.unwrap() }
}

fn process_top() -> usize {
    unsafe { PROCESS_BASE.unwrap() + PROCESS_USED }
}

fn process_max() -> usize {
    unsafe { PROCESS_BASE.unwrap() + PROCESS_SIZE }
}

fn process_init(base_addr: usize) {
    unsafe {
        PROCESS_BASE = Some(base_addr);
        PROCESS_USED = 0;
    }
}

struct RegionMmap;

impl Mmap for RegionMmap {
    unsafe fn mmap(
        addr: Option<usize>,
        len: usize,
        prot: ProtFlags,
        flags: MapFlags,
        offset: usize,
        fd: Option<isize>,
        need_copy: &mut bool,
    ) -> elf_loader::Result<core::ptr::NonNull<c_void>> {
        let map_addr = addr
            .map(|addr| {
                assert!(process_base() <= addr && addr < process_top());
                assert!(flags.contains(MapFlags::MAP_FIXED));
                addr
            })
            .unwrap_or(process_top());

        let ptr = fd
            .map(|fd| unsafe {
                libc::mmap(
                    map_addr as _,
                    len,
                    prot.bits(),
                    flags.union(MapFlags::MAP_FIXED).bits(),
                    fd as i32,
                    offset as _,
                )
            })
            .unwrap_or_else(|| {
                *need_copy = true;
                map_addr as *mut c_void
            });

        if ptr == libc::MAP_FAILED || (fd.is_some() && ptr as usize != map_addr) {
            return Err(map_error("mmap failed"));
        }

        if addr.is_none() {
            unsafe { PROCESS_USED += len };
        }

        println!(
            "mapping {:p} with prot {} and fd {:?}",
            map_addr as *const i8,
            prot.bits(),
            fd
        );

        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }

    unsafe fn mmap_anonymous(
        addr: usize,
        len: usize,
        prot: ProtFlags,
        flags: MapFlags,
    ) -> elf_loader::Result<core::ptr::NonNull<c_void>> {
        let mut copy = false;
        unsafe { Self::mmap(Some(addr), len, prot, flags, 0, None, &mut copy) }
    }

    unsafe fn munmap(
        addr: core::ptr::NonNull<core::ffi::c_void>,
        len: usize,
    ) -> elf_loader::Result<()> {
        let res = unsafe { libc::munmap(addr.as_ptr(), len) };
        if res != 0 {
            return Err(map_error("munmap failed"));
        }
        Ok(())
    }

    unsafe fn mprotect(
        addr: core::ptr::NonNull<core::ffi::c_void>,
        len: usize,
        prot: ProtFlags,
    ) -> elf_loader::Result<()> {
        let res = unsafe { libc::mprotect(addr.as_ptr(), len, prot.bits()) };
        if res != 0 {
            return Err(map_error("mprotect failed"));
        }
        Ok(())
    }

    unsafe fn mmap_reserve(
        _addr: Option<usize>,
        len: usize,
        use_file: bool,
    ) -> elf_loader::Result<NonNull<c_void>> {
        let map_addr = process_top();
        let flags = MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS | MapFlags::MAP_FIXED;
        let prot = if use_file {
            ProtFlags::PROT_NONE
        } else {
            ProtFlags::PROT_WRITE
        };

        let ptr = unsafe { libc::mmap(map_addr as _, len, prot.bits(), flags.bits(), -1, 0) };
        if ptr == libc::MAP_FAILED || ptr as usize != map_addr {
            return Err(map_error("mmap_reserve failed"));
        }

        println!(
            "reserving {:p} with prot {} (len: {})",
            map_addr as *const i8,
            prot.bits(),
            len
        );

        unsafe { PROCESS_USED += len }
        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }
}

#[cold]
#[inline(never)]
fn map_error(msg: &str) -> Error {
    Error::MmapError {
        msg: msg.to_string(),
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

    let mut loader = Loader::<RegionMmap>::new();
    process_init(1024 * 1024 * 1024 * 1024);

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
