#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void entry(void) {
    printf("test_malloc: starting\n");
    
    char *str = malloc(100);
    if (!str) {
        printf("test_malloc: FAIL - malloc failed\n");
        return;
    }
    strcpy(str, "Memory allocation works");
    printf("test_malloc: allocated string: %s\n", str);
    free(str);
    
    int *arr = calloc(10, sizeof(int));
    if (!arr) {
        printf("test_malloc: FAIL - calloc failed\n");
        return;
    }
    int all_zero = 1;
    for (int i = 0; i < 10; i++) {
        if (arr[i] != 0) {
            all_zero = 0;
            break;
        }
    }
    if (!all_zero) {
        printf("test_malloc: FAIL - calloc not zeroed\n");
        free(arr);
        return;
    }
    free(arr);
    
    char *buf = malloc(10);
    strcpy(buf, "Small");
    buf = realloc(buf, 100);
    if (!buf) {
        printf("test_malloc: FAIL - realloc failed\n");
        return;
    }
    strcat(buf, " -> Large");
    printf("test_malloc: realloc: %s\n", buf);
    free(buf);
    
    printf("test_malloc: PASS\n");
}

