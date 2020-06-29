int main() {
    int x = 4;
    int y = 3;

    x += 1;
    y += 3;

    // big, useless loop to test ctrl-c functionality
    for (int i = 0; i < 1024 * 32; i++) {
        x += 1;
    }

    return x;
}
