#include <stdio.h>

// --- CHALLENGE 1: The Constructor ---
// This function must run BEFORE entry(). 
// If your loader jumps straight to entry(), 'init_state' will be 0.
int init_state = 0;

__attribute__((constructor))
void my_constructor() {
    printf("[Constructor] System initializing...\n");
    init_state = 0xBEEF;
}

// --- CHALLENGE 2: The Dispatch Table ---
// This creates a complex relocation scenario.
// The array 'math_ops' is in .data.
// It contains pointers to 'add' and 'sub', which are in .text.
// The loader must patch the values inside this array to point to the new heap addresses.
int add(int a, int b) { return a + b; }
int sub(int a, int b) { return a - b; }

typedef int (*op_func)(int, int);

// This array requires R_X86_64_64 or R_X86_64_RELATIVE relocations 
// to be applied to specific offsets within the .data section.
op_func math_ops[] = { add, sub };

__attribute__((visibility("default")))
void entry() {
    printf("\n=== IMPOSSIBLE TEST ===\n");

    // TEST 1: Check Constructor
    printf("[1] Checking Constructor Execution:\n");
    if (init_state == 0xBEEF) {
        printf("    -> PASS (Constructor ran!)\n");
    } else {
        printf("    -> FAIL (Constructor skipped. Value: %d)\n", init_state);
        printf("       (Your loader jumped to entry without processing .init_array)\n");
    }

    // TEST 2: Check Function Pointer Relocation
    printf("[2] Checking Dispatch Table:\n");
    int val1 = math_ops[0](10, 20); // Should call add
    int val2 = math_ops[1](10, 20); // Should call sub
    
    printf("    add(10, 20) = %d (Expected 30)\n", val1);
    printf("    sub(10, 20) = %d (Expected -10)\n", val2);

    if (val1 == 30 && val2 == -10) {
        printf("    -> PASS\n");
    } else {
        printf("    -> FAIL (Relocation error in global array)\n");
    }

    printf("=== END TEST ===\n\n");
}
