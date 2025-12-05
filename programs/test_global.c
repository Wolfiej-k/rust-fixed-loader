#include <stdio.h>
#include <string.h>

int global_int = 123;
float global_float = 3.14f;
char global_string[] = "Global string data";
static int static_int = 456;

void entry(void) {
    printf("test_global: starting\n");
    
    printf("test_global: global_int = %d\n", global_int);
    printf("test_global: global_float = %.2f\n", global_float);
    printf("test_global: global_string = %s\n", global_string);
    printf("test_global: static_int = %d\n", static_int);
    
    global_int = 999;
    global_float = 2.71f;
    strcpy(global_string, "Modified");
    
    printf("test_global: after modification:\n");
    printf("test_global: global_int = %d\n", global_int);
    printf("test_global: global_float = %.2f\n", global_float);
    printf("test_global: global_string = %s\n", global_string);
    
    printf("test_global: PASS\n");
}
