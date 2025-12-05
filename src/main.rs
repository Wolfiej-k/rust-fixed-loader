use elf_loader::{
    Error, Loader,
    mmap::{MapFlags, Mmap, ProtFlags},
    object::ElfFile,
};
use rlsf::Tlsf;
use std::{
    alloc::Layout, boxed::Box, cell::RefCell, env, ffi::CString, ffi::c_void, process, ptr::NonNull,
};

static PROCESS_SIZE: usize = 16 * 1024 * 1024 * 1024;
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

        if map_addr + len > process_max() {
            return Err(map_error("out of process memory"));
        }

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
        if map_addr + len > process_max() {
            return Err(map_error("out of process memory"));
        }

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

thread_local! {
    static THREAD_HEAP: RefCell<Option<Tlsf<'static, u16, u16, 12, 16>>> = RefCell::new(None);
}

fn resolve_host_symbol(name: &str) -> Option<*const ()> {
    const ALIGNMENT: usize = 16;

    extern "C" fn malloc(size: usize) -> *mut c_void {
        if size == 0 {
            return std::ptr::null_mut();
        }

        let layout = match Layout::from_size_align(size, ALIGNMENT) {
            Ok(l) => l,
            Err(_) => return std::ptr::null_mut(),
        };

        THREAD_HEAP.with(
            |h| match h.borrow_mut().as_mut().unwrap().allocate(layout) {
                Some(nn) => nn.as_ptr() as *mut c_void,
                None => std::ptr::null_mut(),
            },
        )
    }

    extern "C" fn free(ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }

        THREAD_HEAP.with(|h| {
            let nn = NonNull::new(ptr as *mut u8).expect("ptr was null-checked");
            unsafe { h.borrow_mut().as_mut().unwrap().deallocate(nn, ALIGNMENT) };
        });
    }

    extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
        let total = match nmemb.checked_mul(size) {
            Some(t) if t > 0 => t,
            _ => return std::ptr::null_mut(),
        };

        let ptr = malloc(total);
        if !ptr.is_null() {
            unsafe { std::ptr::write_bytes(ptr, 0, total) };
        }
        ptr
    }

    extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
        if ptr.is_null() {
            return malloc(size);
        }

        if size == 0 {
            free(ptr);
            return std::ptr::null_mut();
        }

        let new_layout = match Layout::from_size_align(size, ALIGNMENT) {
            Ok(l) => l,
            Err(_) => return std::ptr::null_mut(),
        };

        THREAD_HEAP.with(|h| {
            let mut guard = h.borrow_mut();
            let tlsf = guard.as_mut().unwrap();
            unsafe {
                let nn_old = match NonNull::new(ptr as *mut u8) {
                    Some(p) => p,
                    None => return std::ptr::null_mut(),
                };

                match tlsf.reallocate(nn_old, new_layout) {
                    Some(nn_new) => nn_new.as_ptr() as *mut c_void,
                    None => std::ptr::null_mut(),
                }
            }
        })
    }

    match name {
        "malloc" => return Some(malloc as *const ()),
        "free" => return Some(free as *const ()),
        "calloc" => return Some(calloc as *const ()),
        "realloc" => return Some(realloc as *const ()),
        _ => {}
    };

    let c_name = CString::new(name).ok()?;
    let addr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_name.as_ptr()) };
    if addr.is_null() {
        None
    } else {
        Some(addr as *const ())
    }
}

fn spawn_with_stack<F>(stack_addr: usize, stack_size: usize, f: F) -> libc::pthread_t
where
    F: FnOnce() + Send + 'static,
{
    extern "C" fn thread_trampoline(data: *mut c_void) -> *mut c_void {
        unsafe {
            let closure_ptr = data as *mut Box<dyn FnOnce() + Send>;
            let closure: Box<dyn FnOnce() + Send> = Box::from_raw(closure_ptr);
            closure();
        }
        std::ptr::null_mut()
    }

    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        libc::pthread_attr_init(&mut attr);
        libc::pthread_attr_setstack(&mut attr, stack_addr as *mut c_void, stack_size);

        let boxed: Box<dyn FnOnce() + Send> = Box::new(f);
        let mut thread: libc::pthread_t = std::mem::zeroed();

        let ret = libc::pthread_create(
            &mut thread,
            &attr,
            thread_trampoline,
            Box::into_raw(Box::new(boxed)) as *mut _,
        );
        assert_eq!(ret, 0);

        libc::pthread_attr_destroy(&mut attr);
        thread
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lib1> <lib2> ...", args[0]);
        process::exit(1);
    }

    let first_region = 1024 * 1024 * 1024 * 1024;
    let mut handles = Vec::new();

    for (idx, path) in args[1..].iter().enumerate() {
        let object = ElfFile::from_path(&path).unwrap();
        process_init(first_region + idx * PROCESS_SIZE);

        let mut loader = Loader::<RegionMmap>::new();
        let lib = loader
            .load_dylib(object, None)
            .unwrap()
            .easy_relocate(&[], &resolve_host_symbol)
            .unwrap();

        let heap_size = 16 * 1024 * 1024;
        let heap_base = unsafe {
            let ptr = RegionMmap::mmap_reserve(None, heap_size, false).unwrap();
            RegionMmap::mprotect(ptr, heap_size, ProtFlags::PROT_READ | ProtFlags::PROT_WRITE)
                .unwrap();
            ptr.as_ptr() as usize
        };

        let stack_size = 8 * 1024 * 1024;
        let stack_base = unsafe {
            let ptr = RegionMmap::mmap_reserve(None, stack_size, false).unwrap();
            RegionMmap::mprotect(
                ptr,
                stack_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )
            .unwrap();
            ptr.as_ptr() as usize
        };

        let handle = spawn_with_stack(stack_base, stack_size, move || {
            let heap_slice: NonNull<[u8]> = unsafe {
                let slice = std::slice::from_raw_parts_mut(heap_base as *mut u8, heap_size);
                NonNull::from(slice)
            };

            let mut tlsf = Tlsf::new();
            unsafe { tlsf.insert_free_block_ptr(heap_slice) };
            THREAD_HEAP.with(|h| {
                *h.borrow_mut() = Some(tlsf);
            });

            let entry = unsafe { lib.get::<fn() -> ()>("entry").unwrap() };
            entry();
        });

        handles.push((handle, heap_base, heap_size, stack_base, stack_size));
    }

    for (handle, heap_base, heap_size, stack_base, stack_size) in handles {
        unsafe {
            libc::pthread_join(handle, std::ptr::null_mut());
            RegionMmap::munmap(NonNull::new_unchecked(heap_base as *mut c_void), heap_size)
                .unwrap();
            RegionMmap::munmap(
                NonNull::new_unchecked(stack_base as *mut c_void),
                stack_size,
            )
            .unwrap();
        }
    }
}
