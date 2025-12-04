#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// --- TEST CASE 1: Initialized Globals (.data section) ---
// If relocation fails, reading this might give garbage or crash.
int global_val = 123456;

// --- TEST CASE 2: Uninitialized Globals (.bss section) ---
// Loaders must allocate memory size > file size and zero it out.
// If your loader only copies file bytes and ignores memsz, this won't be zero.
int bss_array[10];

// --- TEST CASE 3: Structs with Pointers (Heavy Relocation) ---
// This is the "Boss Fight" for loaders. 
// 'my_config' lives in .data. 
// It contains a pointer to "Production Mode" which lives in .rodata.
// The loader MUST calculate: load_base + offset_of_string and update the pointer.
typedef struct {
    int id;
    const char* name;
    int flags;
} Config;

Config my_config = { 1, "Production Mode", 0xFF };

// --- TEST CASE 4: Internal Function Calls ---
// Tests relative jumps within the binary.
int internal_adder(int a, int b) {
    return a + b;
}

__attribute__((visibility("default")))
void entry() {
    printf("\n=== LOADING STRESS TEST ===\n");

    // 1. Test .data
    printf("[1] Global Var Check:\n");
    printf("    Expected: 123456\n");
    printf("    Actual:   %d\n", global_val);
    if (global_val == 123456) printf("    -> PASS\n");
    else printf("    -> FAIL\n");

    // Modify global to ensure we have Write permissions
    global_val++; 

    // 2. Test .bss
    printf("[2] BSS Zero Check:\n");
    int bss_ok = 1;
    for(int i=0; i<10; i++) {
        if (bss_array[i] != 0) {
            printf("    FAIL at index %d: Value is %d (Should be 0)\n", i, bss_array[i]);
            bss_ok = 0;
        }
    }
    if (bss_ok) printf("    -> PASS (All zeros)\n");

    // 3. Test Relocations inside Structs
    printf("[3] Struct Pointer Relocation:\n");
    printf("    Config ID: %d\n", my_config.id);
    
    // IF THIS CRASHES (Segfault), your R_X86_64_RELATIVE logic is wrong.
    // The pointer inside the struct is likely still pointing to offset 0x... 
    // instead of the actual virtual address.
    printf("    Config Name: '%s' (Address: %p)\n", my_config.name, my_config.name);
    
    if (strcmp(my_config.name, "Production Mode") == 0) printf("    -> PASS\n");
    else printf("    -> FAIL (String mismatch)\n");

    // 4. Test Internal Logic
    printf("[4] Internal Function Call:\n");
    int sum = internal_adder(10, 20);
    if (sum == 30) printf("    -> PASS (10+20=30)\n");
    else printf("    -> FAIL (Got %d)\n", sum);

    // 5. Test External Logic (Malloc/Free)
    printf("[5] External Symbol (malloc):\n");
    void* ptr = malloc(128);
    if (ptr != NULL) {
        printf("    -> PASS (Allocated at %p)\n", ptr);
        memset(ptr, 0xAA, 128); // Test write access to heap
        free(ptr);
    } else {
        printf("    -> FAIL (malloc returned NULL)\n");
    }

    printf("=== TEST COMPLETE ===\n\n");
}
