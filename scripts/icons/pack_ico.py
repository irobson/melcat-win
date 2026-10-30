"""Packs PNG images into one .ico (PNG-compressed entries, Vista and later).

    python3 scripts/icons/pack_ico.py out.ico app-16.png app-20.png ... app-256.png
"""
import struct, sys

def size_of(png):
    width, height = struct.unpack(">II", png[16:24])
    return width, height

out, paths = sys.argv[1], sys.argv[2:]
images = [open(p, "rb").read() for p in paths]
header = struct.pack("<HHH", 0, 1, len(images))
offset = 6 + 16 * len(images)
entries, data = b"", b""
for png in images:
    w, h = size_of(png)
    entries += struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(png), offset + len(data))
    data += png
open(out, "wb").write(header + entries + data)
print(f"wrote {out}: {len(images)} images")
