#include <stdio.h>
#include <string.h>
#include <math.h>
#include <time.h>

void entry(void) {
    printf("test_libc: starting\n");
    
    char buf[100];
    strcpy(buf, "Hello");
    strcat(buf, " World");
    printf("test_libc: string = %s (len=%zu)\n", buf, strlen(buf));
    
    double x = 2.0;
    printf("test_libc: sqrt(%.1f) = %.2f\n", x, sqrt(x));
    printf("test_libc: sin(0) = %.2f\n", sin(0.0));
    printf("test_libc: pow(2, 3) = %.1f\n", pow(2.0, 3.0));
    
    time_t now = time(NULL);
    printf("test_libc: time = %ld\n", (long)now);
    
    printf("test_libc: PASS\n");
}
