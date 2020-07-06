#define CPU_ID *((volatile unsigned char*)0xffff4200)

int main() {
    // try switching between threads using `thread 1` and `thread 2`!
    int done = 0;
    int x = 0;

    // diverging paths on each CPU core
    if (CPU_ID == 0xaa) {
        while (!done) {}
        return x;
    } else {
        // big, useless loop to test ctrl-c functionality
        for (int i = 0; i < 1024 * 32; i++) {
            x += 1;
        }
        done = 1;
        // loop forever
        for (;;) {}
    }
}
