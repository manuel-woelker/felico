#include <stdio.h>

int main(int argc, char **argv) {
    fprintf(stderr, "Hello, World!\n");
    return 123;
}

// ./zig.exe cc -o hello.exe hello.c -target x86_64-windows-gnu