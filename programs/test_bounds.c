#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

extern const void *process_base;
extern const void *process_limit;

int global = 42;
int array[100];

int in_bounds(void *ptr) {
    return ptr >= process_base && ptr < process_limit;
}

void entry(void) {
    printf("test_bounds: starting\n");
    printf("test_bounds: base = %p, limit = %p\n", process_base, process_limit);

    if (!in_bounds(entry) || !in_bounds(in_bounds)) {
        printf("test_bounds: FAIL - code out of bounds\n");
        return;
    }

    if (!in_bounds(&global)) {
        printf("test_bounds: FAIL - global variable out of bounds\n");
        return;
    }

    for (int i = 0; i < 100; i++) {
        if (!in_bounds(&array[i])) {
            printf("test_bounds: FAIL - array[%d] out of bounds\n", i);
            return;
        }
    }

    int local = 67;
    if (!in_bounds(&local)) {
        printf("test_bounds: FAIL - local variable out of bounds\n");
        return;
    }

    void *ptr = malloc(128);
    if (!ptr || !in_bounds(ptr)) {
        printf("test_bounds: FAIL - malloc NULL or out of bounds\n");
        return;
    }

    if (!in_bounds(&process_base) || !in_bounds(&process_limit)) {
        printf("test_bounds: FAIL - bounds variables not in bounds\n");
        return;
    }

    printf("test_bounds: PASS\n");
}
