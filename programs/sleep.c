#include <stdio.h>
#include <unistd.h>

__attribute__((visibility("default")))
void entry() {
    sleep(1);
    printf("hello!\n");
}
