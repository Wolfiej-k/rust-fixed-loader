#include <stdio.h>

void entry(void) {
    printf("test_printf: starting\n");
    
    printf("test_printf: integer = %d\n", 42);
    printf("test_printf: hex = 0x%x\n", 0xDEADBEEF);
    printf("test_printf: float = %.2f\n", 3.14159);
    printf("test_printf: string = %s\n", "Hello World");
    printf("test_printf: pointer = %p\n", (void *)entry);
    printf("test_printf: char = %c\n", 'A');
    printf("test_printf: percent = 100%%\n");
    
    printf("test_printf: width = |%10d|\n", 42);
    printf("test_printf: left = |%-10d|\n", 42);
    printf("test_printf: zero = |%010d|\n", 42);
    
    printf("test_printf: PASS\n");
}
