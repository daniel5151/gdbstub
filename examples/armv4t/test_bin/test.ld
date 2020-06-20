ENTRY(main)

MEMORY {
    ram : ORIGIN = 0x55550000, LENGTH = 0x10000000
}

SECTIONS {
    . = 0x55550000;

    .text : ALIGN(4)
    {
        __TEXT_START__ = .;
        *(.text*);
        . = ALIGN(4);
        __TEXT_END__ = .;
    } > ram

    .got : ALIGN(4)
    {
        *(.got*);
    } > ram

    .data : ALIGN(4)
    {
        __DATA_START__ = .;
        *(.data*);
        *(.rodata*);
        __DATA_END__ = .;
    } > ram

    .bss : ALIGN(4)
    {
        __BSS_START__ = .;
        *(.bss*);
        . = ALIGN(4);
        __BSS_END__ = .;
        end = __BSS_END__;
    } > ram

    /DISCARD/ :
    {
        *(.ARM.exidx*) /* index entries for section unwinding */
        *(.ARM.extab*) /* exception unwinding information */
    }
}
