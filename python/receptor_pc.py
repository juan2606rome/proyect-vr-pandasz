import socket
import struct
import threading
import queue
import numpy as np
import cv2
import av

PUERTO_CAMARA = 5001
PUERTO_IMU = 5002

cola_frames = queue.Queue(maxsize=1)
detener_todo = threading.Event()


def recv_exacto(conn, n):
    """TCP es un stream: un solo recv() puede devolver menos de n bytes.
    Hay que leer en bucle hasta juntar exactamente n."""
    buf = bytearray()
    while len(buf) < n:
        chunk = conn.recv(n - len(buf))
        if not chunk:
            return None
        buf.extend(chunk)
    return bytes(buf)


def poner_frame_mas_reciente(frame):
    """Descarta el frame anterior si aún no se mostró, y pone el nuevo."""
    try:
        cola_frames.get_nowait()
    except queue.Empty:
        pass
    try:
        cola_frames.put_nowait(frame)
    except queue.Full:
        pass


def hilo_imu():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", PUERTO_IMU))
    sock.settimeout(0.5)
    print(f"[IMU] escuchando en puerto {PUERTO_IMU}")
    contador = 0
    while not detener_todo.is_set():
        try:
            data, _ = sock.recvfrom(64)
        except socket.timeout:
            continue
        if len(data) == 16:
            contador += 1
            if contador % 30 == 0:
                x, y, z, w = struct.unpack("<ffff", data)
                print(f"head_quat x={x:.4f} y={y:.4f} z={z:.4f} w={w:.4f}", end="\r")


def hilo_camara_red():
    servidor = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    servidor.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    servidor.bind(("0.0.0.0", PUERTO_CAMARA))
    servidor.listen(1)
    servidor.settimeout(1.0)
    print(f"[CAMARA] esperando conexión TCP del celular en puerto {PUERTO_CAMARA}")

    while not detener_todo.is_set():
        try:
            conn, addr = servidor.accept()
        except socket.timeout:
            continue
        print(f"[CAMARA] celular conectado desde {addr}")
        conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

        # Decodificador H.264 EN PROCESO -- sin subprocess, sin pipes de
        # Windows, sin "muxer" que exija timestamps. Le metemos NAL crudos
        # y nos devuelve frames decodificados directo como arrays.
        codec = av.CodecContext.create("h264", "r")
        frames_decodificados = 0

        try:
            while not detener_todo.is_set():
                len_bytes = recv_exacto(conn, 4)
                if len_bytes is None:
                    print("[CAMARA] el celular cerró la conexión")
                    break
                tamano = struct.unpack("<I", len_bytes)[0]
                nal = recv_exacto(conn, tamano)
                if nal is None:
                    print("[CAMARA] el celular cerró la conexión a mitad de un frame")
                    break

                try:
                    paquetes = codec.parse(nal)
                    for paquete in paquetes:
                        for frame in codec.decode(paquete):
                            img = frame.to_ndarray(format="bgr24")
                            poner_frame_mas_reciente(img)
                            frames_decodificados += 1
                            if frames_decodificados % 30 == 0:
                                print(f"[VIDEO] frames decodificados: {frames_decodificados}")
                except Exception as e:
                    # Un NAL corrupto puntual no debería tumbar todo el
                    # decoder -- lo registramos y seguimos con el siguiente.
                    print(f"[VIDEO] frame descartado por error de decode: {e}")
        except (ConnectionResetError, BrokenPipeError) as e:
            print(f"[CAMARA] conexión perdida: {e}")
        finally:
            conn.close()
            if not detener_todo.is_set():
                print("[CAMARA] esperando que el celular vuelva a conectar...")


if __name__ == "__main__":
    threading.Thread(target=hilo_imu, daemon=True).start()
    threading.Thread(target=hilo_camara_red, daemon=True).start()

    # El bucle de ventana (imshow/waitKey) vive en el hilo principal --
    # obligatorio en Windows para que la ventana se refresque bien.
    cv2.namedWindow("Camara del mando (VR)", cv2.WINDOW_NORMAL)
    try:
        while True:
            try:
                frame = cola_frames.get(timeout=0.5)
                cv2.imshow("Camara del mando (VR)", frame)
            except queue.Empty:
                pass
            if cv2.waitKey(1) & 0xFF == ord('q'):
                break
    except KeyboardInterrupt:
        pass
    finally:
        detener_todo.set()
        cv2.destroyAllWindows()