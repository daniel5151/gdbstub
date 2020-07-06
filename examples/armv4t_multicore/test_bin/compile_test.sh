arm-none-eabi-gcc -c test.c -march=armv4t -O0 -g -std=c11 -fdebug-prefix-map=$(pwd)=.
arm-none-eabi-ld -static -Ttest.ld test.o -o test.elf
