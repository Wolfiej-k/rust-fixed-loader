#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NUM_ALLOCS 1000
#define ALLOC_SIZE 1024

void entry(void) {
    printf("test_stress: starting\n");
    
    void *ptrs[NUM_ALLOCS];
    
    for (int i = 0; i < NUM_ALLOCS; i++) {
        ptrs[i] = malloc(ALLOC_SIZE);
        if (!ptrs[i]) {
            printf("test_stress: FAIL - allocation failed at %d\n", i);
            return;
        }
        memset(ptrs[i], i & 0xFF, ALLOC_SIZE);
    }
    printf("test_stress: allocated %d blocks of %d bytes\n", NUM_ALLOCS, ALLOC_SIZE);
    
    int errors = 0;
    for (int i = 0; i < NUM_ALLOCS; i++) {
        unsigned char *p = ptrs[i];
        for (int j = 0; j < ALLOC_SIZE; j++) {
            if (p[j] != (i & 0xFF)) {
                errors++;
                break;
            }
        }
    }
    printf("test_stress: verification errors = %d\n", errors);
    
    for (int i = 0; i < NUM_ALLOCS; i++) {
        free(ptrs[i]);
    }
    
    if (errors == 0) {
        printf("test_stress: PASS\n");
    } else {
        printf("test_stress: FAIL\n");
    }
}
