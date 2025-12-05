#include <stdio.h>

__thread int tls_counter = 0;
__thread char tls_buffer[64] = {0};

void entry(void) {
    printf("test_tls: starting\n");
    
    tls_counter = 42;
    snprintf(tls_buffer, sizeof(tls_buffer), "TLS test string");
    
    printf("test_tls: counter = %d\n", tls_counter);
    printf("test_tls: buffer = %s\n", tls_buffer);
    
    if (tls_counter == 42) {
        printf("test_tls: PASS\n");
    } else {
        printf("test_tls: FAIL\n");
    }
}
