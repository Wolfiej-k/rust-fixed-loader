# fixed_loader

Load `.so` libraries and other `ET_DYN` ELF binaries into fixed memory regions,
and spawn a thread to execute the entry point. Each loaded "process" has a
private heap and stack contiguous with its code and data.

> **WARNING:** This library is highly experimental and relies on
> [elf_loader](https://github.com/weizhiao/rust-elfloader) for its core
> functionality. Expect bugs.

### Features
 - Load `ET_DYN` ELF binaries (shared objects and PIE executables) into a fixed
 virtual address.
 - Create an isolated "process" with its own contiguous code/data/heap/stack.
 - Spawn a thread to execute a designated entry function inside the loaded
 image.
 - Written in Rust, though the codebase is largely `unsafe`.

### Limitations
 - The example in `src/main.rs` only supports `libc` programs by default. The
 C++ standard library can be resolved in the same manner as `libm`, but the
 current heap allocation strategy assumes the `malloc` family.
 - Programs must be linked with `-shared -Wl,-z,now`.
 - Programs must have a dedicated `void entry(void)` function.
 - Thread-local storage does not work.

### Quick Start
Build the loader:
```
cargo build --release
```

Run the example:
```
cargo run --release path/to/program.so
```
