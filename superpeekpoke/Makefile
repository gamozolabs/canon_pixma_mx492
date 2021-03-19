all:
	arm-none-eabi-gcc -mthumb -Os -g -ffreestanding -nostdlib -nostdinc -Wall shellcode.c
	objdump -d ./a.out
	objcopy -O binary --only-section=.text a.out a.bin

