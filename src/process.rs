use crate::LoaderError;
use crate::memory::{RegionMmap, with_region_context};
use crate::runtime::{resolve_host_symbols, set_process_info, set_thread_heap};
use elf_loader::mmap::{Mmap, ProtFlags};
use elf_loader::{Loader, object::ElfFile};
use rlsf::Tlsf;
use std::ffi::c_void;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy)]
pub struct ProcessBounds {
    pub base: usize,
    pub limit: usize,
}

/// Handle to a spawned process.
pub struct ProcessHandle {
    thread: libc::pthread_t,
}

impl ProcessHandle {
    /// Just wraps pthread.
    pub fn new(thread: libc::pthread_t) -> Self {
        Self { thread }
    }

    /// Wait for the process to finish.
    pub fn join(self) {
        unsafe {
            libc::pthread_join(self.thread, std::ptr::null_mut());
        }
    }
}

/// Represents an isolated process with its own memory region.
pub struct Process {
    bounds: ProcessBounds,
}

impl Process {
    /// Initialize new process with given memory bounds.
    pub fn new(base_addr: usize, limit_addr: usize) -> Self {
        Self {
            bounds: ProcessBounds {
                base: base_addr,
                limit: limit_addr,
            },
        }
    }

    /// Spawn a new isolated process from an ELF file.
    pub fn spawn(
        &self,
        elf_path: &str,
        entry_name: &'static str,
        stack_size: usize,
        heap_size: usize,
    ) -> Result<ProcessHandle, LoaderError> {
        // Thankfully elf_loader makes this project slightly less hacky
        let object = ElfFile::from_path(&elf_path).map_err(LoaderError::Elf)?;

        // Since mmap needs globally set bounds
        with_region_context(self.bounds, || {
            // Allocate bounds globals
            let bounds_addr = self.bounds.base;
            let bounds_size = 4096;
            RegionMmap::mmap_next(
                bounds_addr,
                bounds_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )?;

            unsafe {
                let ptr = bounds_addr as *mut usize;
                ptr.write(self.bounds.base);
                ptr.add(1).write(self.bounds.limit);

                // Bounds memory should be read-only now so guest processes
                // can't overwrite their own bounds
                RegionMmap::mprotect(
                    NonNull::new_unchecked(ptr as *mut c_void),
                    bounds_size,
                    ProtFlags::PROT_READ,
                )?;
            }

            // Allocate stack
            let stack_addr = bounds_addr + bounds_size;
            RegionMmap::mmap_next(
                stack_addr,
                stack_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )?;

            // Allocate heap
            let heap_addr = stack_addr + stack_size;
            RegionMmap::mmap_next(
                heap_addr,
                heap_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )?;

            // Load ELF and resolve symbols
            let mut loader = Loader::<RegionMmap>::new();
            let lib = loader
                .load_dylib(object, None)
                .map_err(LoaderError::Elf)?
                .easy_relocate(&[], &resolve_host_symbols)
                .map_err(LoaderError::Elf)?;

            // Spawn thread using horrifying closure trick
            let thread_handle = unsafe {
                let bounds = self.bounds;
                spawn_with_stack(stack_addr, stack_size, move || {
                    // Install heap in thread-local
                    let mut tlsf = Tlsf::new();
                    let slice = std::slice::from_raw_parts_mut(heap_addr as *mut u8, heap_size);
                    tlsf.insert_free_block_ptr(NonNull::from(slice));
                    set_thread_heap(tlsf);

                    // Install bounds in thread-local
                    set_process_info(Some(bounds));

                    // Call entry point
                    let entry = lib.get::<fn() -> ()>(entry_name);
                    if let Some(f) = entry {
                        f();
                    } else {
                        eprintln!("Entry point '{}' not found!", entry_name);
                    }
                })
            };

            Ok(ProcessHandle::new(thread_handle))
        })
    }
}

/// No idea how this works, but it does.
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
