data = open("captura_debug.h264", "rb").read()
print("tamaño del archivo:", len(data), "bytes")
print("start codes de 4 bytes (00 00 00 01):", data.count(b"\x00\x00\x00\x01"))
print("start codes de 3 bytes (00 00 01):", data.count(b"\x00\x00\x01"))