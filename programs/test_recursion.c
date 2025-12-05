#include <stdio.h>

int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

int fibonacci(int n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

void entry(void) {
    printf("test_recursion: starting\n");
    
    int f10 = factorial(10);
    int f15 = factorial(15);
    printf("test_recursion: factorial(10) = %d\n", f10);
    printf("test_recursion: factorial(15) = %d\n", f15);
    
    int fib10 = fibonacci(10);
    int fib15 = fibonacci(15);
    printf("test_recursion: fibonacci(10) = %d\n", fib10);
    printf("test_recursion: fibonacci(15) = %d\n", fib15);
    
    printf("test_recursion: PASS\n");
}
