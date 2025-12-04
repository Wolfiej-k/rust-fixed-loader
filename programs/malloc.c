#include <stdio.h>
#include <stdlib.h>

__attribute__((visibility("default")))
void entry() {
    void* addr = malloc(16);
    printf("malloc: %p, return: %p\n", malloc, addr);
    free(addr);
}
