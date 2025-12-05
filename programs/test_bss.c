#include <stdio.h>
#include <string.h>

char large_buffer[65536];
int uninitialized_array[1000];

void entry(void) {
    printf("test_bss: starting\n");
    
    int non_zero = 0;
    for (int i = 0; i < 1000; i++) {
        if (uninitialized_array[i] != 0) {
            non_zero++;
        }
    }
    printf("test_bss: non-zero values = %d (should be 0)\n", non_zero);
    
    memset(large_buffer, 'X', sizeof(large_buffer));
    printf("test_bss: filled buffer\n");
    printf("test_bss: first = %c, last = %c\n", 
           large_buffer[0], large_buffer[sizeof(large_buffer) - 1]);

    if (non_zero == 0 && large_buffer[0] == 'X') {
        printf("test_bss: PASS\n");
    } else {
        printf("test_bss: FAIL\n");
    }
}
