import zlib, struct

class Firmware:
    def __init__(self):
        # Read the entire firmware
        with open("firmware_dumps/canon_pixma_mx492.flashbin.BIN", "rb") as fd:
            self.firmware = fd.read()

        # The start of compressed data
        self.compressed_ptr = 0x25000

    def decompress_next(self):
        # Read the length of the compressed payload
        length = \
            struct.unpack("<I", self.firmware[self.compressed_ptr:][:4])[0]

        # Get the compressed data
        compressed = self.firmware[self.compressed_ptr + 4:][:length]

        # Decompress the data
        decompressed = zlib.decompress(compressed)

        # Advance the compressed pointer
        self.compressed_ptr = self.compressed_ptr + 4 + length

        # 4-byte up align the pointer
        self.compressed_ptr = (self.compressed_ptr + 3) & ~3

        return decompressed

    def decompress(self):
        decompressed = bytes()
        decompressed += firmware.decompress_next()
        decompressed += firmware.decompress_next()
        decompressed += firmware.decompress_next()
        decompressed += firmware.decompress_next()
        return decompressed

firmware = Firmware()

with open("decompressed.bin", "wb") as fd:
    fd.write(firmware.decompress())

