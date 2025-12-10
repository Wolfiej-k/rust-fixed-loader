use crate::process::ProcessBounds;
use rlsf::Tlsf;
use std::alloc::Layout;
use std::cell::RefCell;
use std::ffi::{CString, c_void};
use std::ptr::NonNull;

/// Serves allocations up to 64 MiB.
type Heap = Tlsf<'static, u32, u32, 20, 4>;

thread_local! {
    static THREAD_HEAP: RefCell<Option<Heap>> = RefCell::new(None);
    static PROCESS_BOUNDS: RefCell<Option<ProcessBounds>> = RefCell::new(None);
}

/// Set fixed-location heap for current thread.
pub fn set_thread_heap(heap: Heap) {
    THREAD_HEAP.with(|h| *h.borrow_mut() = Some(heap));
}

/// Set process memory bounds for current thread.
pub fn set_process_info(bounds: Option<ProcessBounds>) {
    PROCESS_BOUNDS.with(|p| *p.borrow_mut() = bounds);
}

const ALIGNMENT: usize = 16;

/// Host wrapper for malloc.
extern "C" fn host_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = match Layout::from_size_align(size, ALIGNMENT) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };

    THREAD_HEAP.with(|h| {
        let mut guard = h.borrow_mut();
        match guard.as_mut().and_then(|tlsf| tlsf.allocate(layout)) {
            Some(nn) => nn.as_ptr() as *mut c_void,
            None => std::ptr::null_mut(),
        }
    })
}

/// Host wrapper for free.
extern "C" fn host_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    THREAD_HEAP.with(|h| {
        if let Some(tlsf) = h.borrow_mut().as_mut() {
            if let Some(nn) = NonNull::new(ptr as *mut u8) {
                unsafe { tlsf.deallocate(nn, ALIGNMENT) };
            }
        }
    });
}

/// Host wrapper for calloc.
extern "C" fn host_calloc(nmemb: usize, size: usize) -> *mut c_void {
    let total = match nmemb.checked_mul(size) {
        Some(t) => t,
        None => return std::ptr::null_mut(),
    };

    let ptr = host_malloc(total);
    if !ptr.is_null() {
        unsafe { std::ptr::write_bytes(ptr, 0, total) };
    }

    ptr
}

/// Host wrapper for realloc.
extern "C" fn host_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return host_malloc(size);
    }

    if size == 0 {
        host_free(ptr);
        return std::ptr::null_mut();
    }

    let layout = match Layout::from_size_align(size, ALIGNMENT) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };

    THREAD_HEAP.with(|h| {
        let mut guard = h.borrow_mut();
        let tlsf = match guard.as_mut() {
            Some(t) => t,
            None => return std::ptr::null_mut(),
        };

        unsafe {
            match NonNull::new(ptr as *mut u8) {
                Some(nn_old) => tlsf
                    .reallocate(nn_old, layout)
                    .map(|n| n.as_ptr() as *mut c_void)
                    .unwrap_or(std::ptr::null_mut()),
                None => std::ptr::null_mut(),
            }
        }
    })
}

/// Resolve libc symbols from the host. The malloc family is patched so
/// processes have private thread-local heaps.
pub fn resolve_host_symbols(name: &str) -> Option<*const ()> {
    match name {
        "malloc" => Some(host_malloc as *const ()),
        "free" => Some(host_free as *const ()),
        "calloc" => Some(host_calloc as *const ()),
        "realloc" => Some(host_realloc as *const ()),
        "process_base" => PROCESS_BOUNDS.with(|p| p.borrow().map(|b| b.base as *const ())),
        "process_limit" => PROCESS_BOUNDS.with(|p| {
            p.borrow()
                .map(|b| (b.base + std::mem::size_of::<usize>()) as *const ())
        }),
        _ => {
            let c_name = CString::new(name).ok()?;
            let addr = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_name.as_ptr()) };
            if addr.is_null() {
                None
            } else {
                Some(addr as *const ())
            }
        }
    }
}
