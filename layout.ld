MEMORY
{
	ITCM (rwx):  ORIGIN = 0x00000000, LENGTH = 512K
	DTCM (rwx):  ORIGIN = 0x20000000, LENGTH = 512K
	RAM (rwx):   ORIGIN = 0x20200000, LENGTH = 512K
	FLASH (rwx): ORIGIN = 0x60000000, LENGTH = 1984K
}

ENTRY(_IMAGE_VECTOR_TABLE)

SECTIONS
{
	.text.progmem : {
		KEEP(*(.flashconfig))
		FILL(0xFF)
		. = ORIGIN(FLASH) + 0x1000;
		KEEP(*(.ivt))
		KEEP(*(.bootdata))
		KEEP(*(.vectors))
		KEEP(*(.startup))
		*(.text*)
		*(.rodata*)
		. = ALIGN(16);
	} > FLASH

	.bss ALIGN(4) : {
		*(.bss*)
		*(COMMON)
	} > DTCM

	_image_length = SIZEOF(.text.progmem);

	_bss_start = ADDR(.bss);
	_bss_end = ADDR(.bss) + SIZEOF(.bss);

        /* We define the boot stack position here, since Rust makes it
	hard to mix function pointers and integers in a single
	array. */
	_boot_stack = 0x20010000;

	/DISCARD/ : {
                *(.ARM.exidx)
        }
}
