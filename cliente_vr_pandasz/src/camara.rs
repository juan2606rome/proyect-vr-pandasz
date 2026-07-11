#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

use android_activity::AndroidApp;
use log::{error, info};
use std::ffi::{c_void, CString};
use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/camera_bindings.rs"));

const TEMPLATE_RECORD: u32 = 3;
const CONFIGURE_FLAG_ENCODE: u32 = 1;
const BUFFER_FLAG_CODEC_CONFIG: u32 = 2; // marca el buffer que contiene SPS/PPS

// --- CONFIGURA ESTO (misma IP que en lib.rs) ---
const IP_PC: &str = "192.168.1.7";
const PUERTO_CAMARA: u16 = 5001;
// -------------------------------------------------

unsafe fn set_str(fmt: *mut AMediaFormat, key: &str, val: &str) {
    let k = CString::new(key).unwrap();
    let v = CString::new(val).unwrap();
    AMediaFormat_setString(fmt, k.as_ptr(), v.as_ptr());
}
unsafe fn set_i32(fmt: *mut AMediaFormat, key: &str, val: i32) {
    let k = CString::new(key).unwrap();
    AMediaFormat_setInt32(fmt, k.as_ptr(), val);
}

fn esperar_permiso_camara(app: &AndroidApp) -> bool {
    for _ in 0..60 {
        if crate::permiso_camara_concedido(app) {
            info!("Permiso de cámara concedido, continuando...");
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    error!("Timeout esperando permiso de cámara (30s)");
    false
}

fn conectar_tcp(activo: &Arc<AtomicBool>) -> Option<TcpStream> {
    while activo.load(Ordering::Relaxed) {
        match TcpStream::connect((IP_PC, PUERTO_CAMARA)) {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                info!("TCP de cámara conectado a {}:{}", IP_PC, PUERTO_CAMARA);
                return Some(s);
            }
            Err(e) => {
                error!("No se pudo conectar TCP a {}:{} -> {:?}, reintentando en 1s...", IP_PC, PUERTO_CAMARA, e);
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
    None
}

/// Envía un NAL (o el bloque de SPS/PPS) con el protocolo [4 bytes tamaño][datos].
fn enviar_nal(stream: &mut TcpStream, datos: &[u8]) -> std::io::Result<()> {
    let len_bytes = (datos.len() as u32).to_le_bytes();
    stream.write_all(&len_bytes)?;
    stream.write_all(datos)
}

pub fn iniciar_camara(app: &AndroidApp, activo: Arc<AtomicBool>) {
    if !esperar_permiso_camara(app) {
        return;
    }

    let mut stream = match conectar_tcp(&activo) {
        Some(s) => s,
        None => return,
    };

    // Guarda el bloque de SPS/PPS (BUFFER_FLAG_CODEC_CONFIG) la primera vez
    // que sale del encoder, para poder reenviarlo cada vez que el TCP se
    // reconecta a un ffmpeg nuevo -- sin esto, cualquier ffmpeg que no sea
    // el de la primerísima conexión nunca ve el SPS/PPS y falla siempre.
    let mut csd: Vec<u8> = Vec::new();

    unsafe {
        let format = AMediaFormat_new();
        set_str(format, "mime", "video/avc");
        set_i32(format, "width", 640);
        set_i32(format, "height", 480);
        set_i32(format, "bitrate", 4_000_000);
        set_i32(format, "frame-rate", 30);
        set_i32(format, "color-format", 0x7f000789);
        set_i32(format, "i-frame-interval", 2); 

                // --- Hints de baja latencia, estilo ALVR ---
        set_i32(format, "profile", 0x01);       // AVCProfileBaseline: sin B-frames, sin reordenamiento
        set_i32(format, "bitrate-mode", 2);     // BITRATE_MODE_CBR: bitrate constante, menos análisis interno que VBR
        set_i32(format, "priority", 0);         // 0 = tiempo real (evita que el encoder optimice para "calidad offline")
        set_i32(format, "latency", 0);          // hint directo: minimizar frames de retraso internos (varios encoders MediaTek lo respetan)

        let mime = CString::new("video/avc").unwrap();
        let codec = AMediaCodec_createEncoderByType(mime.as_ptr());
        if codec.is_null() {
            error!("No se pudo crear el encoder H.264");
            return;
        }
        if AMediaCodec_configure(codec, format, std::ptr::null_mut(), std::ptr::null_mut(), CONFIGURE_FLAG_ENCODE) != 0 {
            error!("Fallo configurando el encoder");
            return;
        }
        let mut input_surface: *mut ANativeWindow = std::ptr::null_mut();
        if AMediaCodec_createInputSurface(codec, &mut input_surface) != 0 || input_surface.is_null() {
            error!("No se pudo crear la input surface del encoder");
            return;
        }
        if AMediaCodec_start(codec) != 0 {
            error!("Fallo iniciando el encoder");
            return;
        }
        info!("Encoder H.264 iniciado correctamente");

        let mgr = ACameraManager_create();
        if mgr.is_null() {
            error!("No se pudo crear ACameraManager");
            return;
        }
        let mut id_list: *mut ACameraIdList = std::ptr::null_mut();
        if ACameraManager_getCameraIdList(mgr, &mut id_list) != 0 || id_list.is_null() {
            error!("No se pudo listar cámaras");
            return;
        }
        let num_camaras = (*id_list).numCameras;
        info!("Cámaras detectadas: {}", num_camaras);
        if num_camaras == 0 {
            error!("Este dispositivo no reporta ninguna cámara");
            return;
        }
        let camera_id_ptr = *(*id_list).cameraIds;

        let mut device: *mut ACameraDevice = std::ptr::null_mut();
        let mut device_callbacks = ACameraDevice_StateCallbacks {
            context: std::ptr::null_mut(),
            onDisconnected: Some(on_device_disconnected),
            onError: Some(on_device_error),
        };
        let status = ACameraManager_openCamera(mgr, camera_id_ptr, &mut device_callbacks, &mut device);
        if status != 0 || device.is_null() {
            error!("No se pudo abrir la cámara, status={}", status);
            return;
        }
        info!("Cámara abierta correctamente");

        let mut output_container: *mut ACaptureSessionOutputContainer = std::ptr::null_mut();
        ACaptureSessionOutputContainer_create(&mut output_container);
        let mut session_output: *mut ACaptureSessionOutput = std::ptr::null_mut();
        ACaptureSessionOutput_create(input_surface, &mut session_output);
        ACaptureSessionOutputContainer_add(output_container, session_output);

        let mut request: *mut ACaptureRequest = std::ptr::null_mut();
        ACameraDevice_createCaptureRequest(device, TEMPLATE_RECORD, &mut request);
        let mut target: *mut ACameraOutputTarget = std::ptr::null_mut();
        ACameraOutputTarget_create(input_surface, &mut target);
        ACaptureRequest_addTarget(request, target);

        let mut session: *mut ACameraCaptureSession = std::ptr::null_mut();
        let mut session_callbacks = ACameraCaptureSession_stateCallbacks {
            context: std::ptr::null_mut(),
            onClosed: Some(on_session_closed),
            onReady: Some(on_session_ready),
            onActive: Some(on_session_active),
        };
        let status = ACameraDevice_createCaptureSession(device, output_container, &mut session_callbacks, &mut session);
        if status != 0 || session.is_null() {
            error!("No se pudo crear la sesión de captura, status={}", status);
            return;
        }
        let mut request_ptr = request;
        if ACameraCaptureSession_setRepeatingRequest(session, std::ptr::null_mut(), 1, &mut request_ptr, std::ptr::null_mut()) != 0 {
            error!("Fallo iniciando captura repetitiva");
            return;
        }
        info!("Captura de cámara iniciada, transmitiendo por TCP...");

        let mut info: AMediaCodecBufferInfo = std::mem::zeroed();
        let mut frames_enviados: u64 = 0;
        'captura: while activo.load(Ordering::Relaxed) {
            let idx = AMediaCodec_dequeueOutputBuffer(codec, &mut info, 10_000);
            if idx >= 0 {
                let mut out_size: usize = 0;
                let ptr = AMediaCodec_getOutputBuffer(codec, idx as usize, &mut out_size);
                if !ptr.is_null() && info.size > 0 {
                    let slice = std::slice::from_raw_parts(ptr.add(info.offset as usize), info.size as usize);

                    // Si este buffer ES el SPS/PPS, guárdalo para poder
                    // reenviarlo en futuras reconexiones.
                    if info.flags & BUFFER_FLAG_CODEC_CONFIG != 0 {
                        csd.clear();
                        csd.extend_from_slice(slice);
                        info!("SPS/PPS capturado ({} bytes)", csd.len());
                    }

                    let resultado = enviar_nal(&mut stream, slice);
                    match resultado {
                        Ok(_) => {
                            frames_enviados += 1;
                            if frames_enviados % 60 == 0 {
                                info!("Frames enviados: {}", frames_enviados);
                            }
                        }
                        Err(e) => {
                            error!("Conexión TCP caída ({:?}), reconectando...", e);
                            AMediaCodec_releaseOutputBuffer(codec, idx as usize, false);
                            match conectar_tcp(&activo) {
                                Some(mut s) => {
                                    // Reenvía el SPS/PPS guardado ANTES que
                                    // cualquier frame nuevo, para que el
                                    // ffmpeg nuevo pueda decodificar desde ya.
                                    if !csd.is_empty() {
                                        if let Err(e) = enviar_nal(&mut s, &csd) {
                                            error!("No se pudo reenviar SPS/PPS: {:?}", e);
                                        } else {
                                            info!("SPS/PPS reenviado a la nueva conexión");
                                        }
                                    }
                                    stream = s;
                                }
                                None => break 'captura,
                            }
                            continue;
                        }
                    }
                }
                AMediaCodec_releaseOutputBuffer(codec, idx as usize, false);
            }
        }

        info!("Cerrando módulo de cámara (señal de salida recibida)");
        ACameraCaptureSession_stopRepeating(session);
        AMediaCodec_stop(codec);
        AMediaCodec_delete(codec);
        ACameraCaptureSession_close(session);
        ACameraDevice_close(device);
    }
}

extern "C" fn on_device_disconnected(_ctx: *mut c_void, _dev: *mut ACameraDevice) { error!("Cámara desconectada"); }
extern "C" fn on_device_error(_ctx: *mut c_void, _dev: *mut ACameraDevice, err: i32) { error!("Error de cámara, código={}", err); }
extern "C" fn on_session_closed(_ctx: *mut c_void, _s: *mut ACameraCaptureSession) { info!("Sesión de captura cerrada"); }
extern "C" fn on_session_ready(_ctx: *mut c_void, _s: *mut ACameraCaptureSession) { info!("Sesión de captura lista"); }
extern "C" fn on_session_active(_ctx: *mut c_void, _s: *mut ACameraCaptureSession) { info!("Sesión de captura activa"); }