/// gcc -m32 decompress_rle.c

#include <stdlib.h>
#include <stdio.h>

typedef unsigned char byte;
typedef unsigned int uint;

byte *DAT_f02602d4 = NULL;
byte *DAT_184d5000 = NULL;

void FUN_f002024c(void)
{
  byte bVar1;
  byte *pbVar2;
  byte *pbVar3;
  byte *pbVar4;
  uint uVar5;
  int iVar6;
  uint uVar7;
  uint uVar8;
  
  pbVar2 = DAT_f02602d4;
  pbVar4 = DAT_184d5000;
                    /* WARNING: Treating indirect jump as call */
  do {
    uVar5 = (uint)*pbVar2;
    pbVar3 = pbVar2 + 1;
    uVar7 = uVar5 & 3;
    if ((*pbVar2 & 3) == 0) {
      uVar7 = (uint)*pbVar3;
      pbVar3 = pbVar2 + 2;
    }
    uVar8 = (int)uVar5 >> 4;
    if (uVar8 == 0) {
      uVar8 = (uint)*pbVar3;
      pbVar3 = pbVar3 + 1;
    }
    while (uVar7 = uVar7 - 1, uVar7 != 0) {
      bVar1 = *pbVar3;
      pbVar3 = pbVar3 + 1;
      *pbVar4 = bVar1;
      pbVar4 = pbVar4 + 1;
    }
    pbVar2 = pbVar3;
    if (uVar8 != 0) {
      pbVar2 = pbVar3 + 1;
      uVar7 = (uVar5 << 0x1c) >> 0x1e;
      if (uVar7 == 3) {
        uVar7 = (uint)*pbVar2;
        pbVar2 = pbVar3 + 2;
      }
      pbVar3 = pbVar4 + (uVar7 * -0x100 - (uint)*pbVar3);
      iVar6 = uVar8 + 1;
      do {
        bVar1 = *pbVar3;
        pbVar3 = pbVar3 + 1;
        *pbVar4 = bVar1;
        pbVar4 = pbVar4 + 1;
        iVar6 = iVar6 + -1;
      } while (-1 < iVar6);
    }
  } while (pbVar4 < (byte *)(DAT_184d5000 + (0x185067c0 - 0x184d5000)));
  printf("Read 0x%x compressed bytes\n", pbVar2 - DAT_f02602d4);
  return;
}

// src, dest, len
void FUN_f028030c(byte *param_1,byte *param_2,int param_3)

{
  byte bVar1;
  byte *pbVar2;
  byte *pbVar3;
  uint uVar4;
  int iVar5;
  uint uVar6;
  uint uVar7;

  byte *orig_param1 = param_1;
  
  pbVar3 = param_2 + param_3;
  do {
    uVar4 = (uint)*param_1;
    pbVar2 = param_1 + 1;
    uVar6 = uVar4 & 3;
    if ((*param_1 & 3) == 0) {
      uVar6 = (uint)*pbVar2;
      pbVar2 = param_1 + 2;
    }
    uVar7 = (int)uVar4 >> 4;
    if (uVar7 == 0) {
      uVar7 = (uint)*pbVar2;
      pbVar2 = pbVar2 + 1;
    }
    while (uVar6 = uVar6 - 1, uVar6 != 0) {
      bVar1 = *pbVar2;
      pbVar2 = pbVar2 + 1;
      *param_2 = bVar1;
      param_2 = param_2 + 1;
    }
    param_1 = pbVar2;
    if (uVar7 != 0) {
      param_1 = pbVar2 + 1;
      uVar6 = (uVar4 << 0x1c) >> 0x1e;
      if (uVar6 == 3) {
        uVar6 = (uint)*param_1;
        param_1 = pbVar2 + 2;
      }
      pbVar2 = param_2 + (uVar6 * -0x100 - (uint)*pbVar2);
      iVar5 = uVar7 + 1;
      do {
        bVar1 = *pbVar2;
        pbVar2 = pbVar2 + 1;
        *param_2 = bVar1;
        param_2 = param_2 + 1;
        iVar5 = iVar5 + -1;
      } while (-1 < iVar5);
    }
  } while (param_2 < pbVar3);

  printf("Read 0x%x compressed bytes\n", param_1 - orig_param1);
  return;
}


int main(void)
{
	if(sizeof(void*) != 4) {
		printf("Yo, 32-bit your shit `-m32` or omsething u kno?\n");
		return -1;
	}

	// Allocate ram
	DAT_184d5000 = calloc(1, 0x185067c0 - 0x184d5000);
	if(!DAT_184d5000) {
		perror("malloc() error ");
		return -1;
	}

	// Allocate room for firmware
	char *firmware = calloc(1, 16 * 1024 * 1024);
	if(!firmware) {
		perror("malloc() error ");
		return -1;
	}

	// Open firmware
	//FILE *fd = fopen("../firmware_dumps/canon_pixma_mx492.flashbin.BIN", "rb");
	FILE *fd = fopen("/home/pleb/Downloads/moose.BIN", "rb");
	if(!fd) {
		perror("fopen() error ");
		return -1;
	}

	// Read entire firmware
	if(fread(firmware, 1, 16 * 1024 * 1024, fd) != 16 * 1024 * 1024) {
		perror("fread() error ");
		return -1;
	}

	// Set up global pointer to offset in firmware with payload
	DAT_f02602d4 = firmware + 0x2602d4;

	// Perform the decompression
	FUN_f002024c();

	fd = fopen("decompressed_rle.bin", "wb");
	if(!fd) {
		perror("fopen() error ");
		return -1;
	}

	// Write out the ram
	if(fwrite(DAT_184d5000, 1, 0x185067c0 - 0x184d5000, fd) != 0x185067c0 - 0x184d5000) {
		perror("fwrite() error ");
		return -1;
	}

	// Decompress the payload implemented at `0xf028088e`
	char *decompressed_large = malloc(0xfe5b20);
	if(!decompressed_large) {
		perror("malloc() error ");
		return -1;
	}

	// Decompress it
	FUN_f028030c(firmware + 0x281000, decompressed_large, 0xfe5b20);
	
	fd = fopen("decompressed_large.bin", "wb");
	if(!fd) {
		perror("fopen() error ");
		return -1;
	}

	// Write out the ram
	if(fwrite(decompressed_large, 1, 0xfe5b20, fd) != 0xfe5b20) {
		perror("fwrite() error ");
		return -1;
	}

	return 0;
}

