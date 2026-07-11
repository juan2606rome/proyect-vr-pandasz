import socket
import struct
import subprocess
import threading
import numpy as np
import cv2

PUERTO_CAMARA = 5001
PUERTO_IMU = 5002
ANCHO = 640
ALTO = 480

# ---------- Hilo IMU: solo imprime el head_quat que llega ----------
def hilo_imu():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", PUERTO_IMU))
    print(f"[IMU] escuchando en puerto {PUERTO_IMU}")
    while True:
        data, _ = sock.recvfrom(64)
        if len(data) == 16:
            x, y, z, w = struct.unpack("<ffff", data)
            # Aquí es donde conectarías esto con tu lógica de tracking
            # del mando en vez de solo imprimir.
            print(f"head_quat x={x:.4f} y={y:.4f} z={z:.4f} w={w:.4f}", end="\r")

# ---------- Hilo cámara: recibe NAL crudos y los pasa a ffmpeg ----------
def hilo_camara():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", PUERTO_CAMARA))
    sock.settimeout(5.0)
    print(f"[CAMARA] escuchando en puerto {PUERTO_CAMARA}")

    ffmpeg_cmd = [
        "ffmpeg",
        "-f", "h264",
        "-i", "pipe:0",
        "-f", "rawvideo",
        "-pix_fmt", "bgr24",
        "pipe:1",
    ]
    proc = subprocess.Popen(
        ffmpeg_cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )

    def leer_frames():
        frame_size = ANCHO * ALTO * 3
        while True:
            raw = proc.stdout.read(frame_size)
            if len(raw) != frame_size:
                break
            frame = np.frombuffer(raw, dtype=np.uint8).reshape((ALTO, ANCHO, 3))
            cv2.imshow("Cámara del mando (VR)", frame)
            if cv2.waitKey(1) & 0xFF == ord('q'):
                break

    t = threading.Thread(target=leer_frames, daemon=True)
    t.start()

    try:
        while True:
            # Protocolo: [4 bytes tamaño little-endian][NAL]
            len_bytes, _ = sock.recvfrom(4)
            if len(len_bytes) != 4:
                continue
            tamano = struct.unpack("<I", len_bytes)[0]
            nal, _ = sock.recvfrom(tamano + 100)  # margen por si el NAL es grande
            proc.stdin.write(nal)
            proc.stdin.flush()
    except socket.timeout:
        print("[CAMARA] timeout esperando datos, ¿el celular sigue transmitiendo?")
    except KeyboardInterrupt:
        pass
    finally:
        proc.stdin.close()
        proc.terminate()

if __name__ == "__main__":
    threading.Thread(target=hilo_imu, daemon=True).start()
    hilo_camara()