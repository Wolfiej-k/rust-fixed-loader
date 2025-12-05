#include <stdio.h>
#include <unistd.h>

__attribute__((visibility("default")))
void entry() {
    FILE *maps = fopen("/proc/self/maps", "r");
    if (!maps) {
        perror("fopen");
        return;
    }

    printf("Virtual Memory Areas (VMAs) for PID %d:\n", getpid());
    printf("%-18s %-18s %s %s %8s %s\n", 
           "Start", "End", "Perms", "Offset", "Dev", "Path");
    printf("================================================================\n");

    char line[512];
    while (fgets(line, sizeof(line), maps)) {
        printf("%s", line);
    }

    fclose(maps);
}
