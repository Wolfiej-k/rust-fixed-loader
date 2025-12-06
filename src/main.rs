use fixed_loader::process::Process;
use std::env;
use std::ffi::CString;

const PROCESS_OFFSET_STEP: usize = 1024 * 1024 * 1024 * 1024;
const PROCESS_SIZE: usize = 16 * 1024 * 1024 * 1024;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <lib1> <lib2> ...", args[0]);
        std::process::exit(1);
    }

    // Pre-load libm so symbols are available to guests
    unsafe {
        let libm = CString::new("libm.so.6").unwrap();
        libc::dlopen(libm.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
    }

    let mut handles = Vec::new();
    for (i, path) in args[1..].iter().enumerate() {
        // Assign a unique base address
        let base_addr = PROCESS_OFFSET_STEP + (i * PROCESS_SIZE);
        let proc = Process::new(base_addr, base_addr + PROCESS_SIZE);

        // Fixed stack and heap sizes for now
        let stack_size = 8 * 1024 * 1024;
        let heap_size = 64 * 1024 * 1024;

        // Spawn guest process
        match proc.spawn(path, "entry", stack_size, heap_size) {
            Ok(handle) => handles.push(handle),
            Err(e) => eprintln!("Failed to spawn {}: {:?}", path, e),
        }
    }

    for handle in handles {
        handle.join();
    }
}
