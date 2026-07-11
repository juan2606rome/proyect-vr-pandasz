Proyecto: Cliente VR propio (Cardboard-style) con módulo de cámara + IMU sobre ALVR client_core

Documento único de estado. Reemplaza a los dos markdown anteriores ("Plan modulo camara alvr.md" y el estado suelto que traía el usuario). Todo lo confirmado en hardware real está marcado como tal; todo lo asumido o no verificado está marcado explícitamente como pendiente o no confirmado, para que cualquier persona (o IA) que retome esto sepa exactamente qué es hecho vs. qué es plan.

1. Objetivo del proyecto

Construir una app Android nativa en Rust (NativeActivity, sin JVM propia más allá de llamadas JNI puntuales) que:


Renderice video en modo Cardboard (split-screen estéreo simple, sin corrección de distorsión de lente todavía).
Lea la orientación de cabeza desde el sensor de rotación del propio celular (sin runtime OpenXR, porque el celular del usuario no lo tiene).
Capture video de la cámara física del celular, lo codifique en H.264 por hardware, y lo transmita por UDP a un PC en la misma red Wi-Fi, para usarlo como cámara de "mando" en un sistema de tracking externo que corre en Python en ese PC.
Eventualmente consuma alvr_client_core (la librería de red/protocolo/decodificación de video de ALVR, licencia MIT) para recibir el video real de SteamVR — esta parte todavía NO se ha empezado.


Restricción de licencia del usuario: todo el código propio debe quedar en MIT. Se descartó explícitamente basarse en PhoneVR (el software que sí corre hoy en el celular del usuario como visor Cardboard) porque es GPL-3.0; PhoneVR solo se usó como referencia conceptual de qué hace, nunca se copió código de ahí.

2. Entorno de desarrollo (Windows) — versiones y rutas exactas confirmadas


Sistema operativo: Windows 10.0.26200.
Proyecto raíz: C:\proyect-vr-pandasz.
Subcarpeta del cliente Rust: C:\proyect-vr-pandasz\cliente_vr_pandasz.
Subcarpeta del repo ALVR clonado (para sacar client_core más adelante): C:\proyect-vr-pandasz\ALVR.
Subcarpeta nueva del receptor Python: C:\proyect-vr-pandasz\python (contiene receptor_pc.py).
Android NDK instalado vía Android Studio: versión 27.1.12297006. Nota: ALVR en su CI fija 25.1.8937393; si algún build futuro contra ALVR falla por versión de NDK, instalar esa versión específica adicional desde Android Studio (SDK Tools → Show Package Details).
Rust: cargo 1.97.0 vía rustup.
Targets de Rust instalados: aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android, i686-linux-android (el celular real del usuario solo necesita aarch64-linux-android, pero los cuatro quedaron instalados).
Herramientas cargo instaladas: cargo-ndk, cargo-apk.
Variables de entorno permanentes (puestas con setx, ya persistentes entre sesiones de terminal): ANDROID_HOME apuntando al SDK de Android bajo LOCALAPPDATA, ANDROID_NDK_HOME apuntando a la carpeta de la versión de NDK instalada, JAVA_HOME apuntando al JDK embebido de Android Studio (carpeta jbr, no jre, en esta instalación).
Celular real usado para pruebas: marca Vivo, Android API level 35 (sdk_version: 35 visto en logcat), idioma/región es_CO.
Python en el PC: C:\Python314\python.exe.
Librerías Python instaladas: opencv-python, numpy.
ffmpeg: en proceso de instalación en el PC (era el bloqueo activo al cierre de este documento — ver sección 7, "Siguiente paso inmediato").


3. Configuración del proyecto Rust (cliente_vr_pandasz)


Nombre del paquete Android: com.pandasz.clientevr.
Nombre del crate Rust: cliente_vr_pandasz.
Tipo de crate: cdylib (requerido por cargo-apk).
min_sdk_version: 26. target_sdk_version: 33.
build_targets en metadata de Android: solo aarch64-linux-android (para builds rápidos de desarrollo; los otros targets de rustup quedan instalados por si se necesitan a futuro).
Orientación fijada en el manifest generado: landscape.
Permisos declarados: android.permission.CAMERA, android.permission.INTERNET.
Feature de hardware declarada como requerida: android.hardware.camera.
Dependencias y versiones resueltas: android-activity en línea mayor 0.6 (resolvió a 0.6.1), ndk 0.9, ndk-sys 0.6, log 0.4, android_logger 0.14, khronos-egl versión mayor 6 con feature dynamic, glow 0.13, libloading 0.8, jni 0.21. Dependencia de build: bindgen 0.69 (usada en build.rs para generar bindings propios contra headers de cámara del NDK, porque el crate ndk no tiene binding maduro para ACameraManager/ACameraDevice/ACameraCaptureSession/AImageReader).
Archivos actuales del proyecto: Cargo.toml, build.rs, wrapper.h (header de entrada para bindgen contra los headers de cámara del NDK), src/lib.rs, src/renderer.rs, src/camara.rs. El código completo vive en el repo del usuario en GitHub (juan2606rome/proyect-vr-pandasz), no se repite aquí.
Keystore de firma de debug usado por cargo apk build: el default de C:\Users\Panda\.android\debug.keystore.


4. Qué está confirmado funcionando en hardware real (celular Vivo)


El esqueleto compila con cargo ndk -t arm64-v8a check y con cargo apk build, instala con adb install y arranca sin excepción fatal de Java (AndroidRuntime: FATAL EXCEPTION nunca apareció en ningún logcat capturado).
El logger (android_logger) muestra correctamente los info!() propios en logcat bajo el tag ClienteVrPandasz; el bug inicial era que android_logger::Config::default() no fija nivel máximo por defecto y filtraba los mensajes — se corrigió agregando .with_max_level(log::LevelFilter::Trace).
Renderer EGL/GL (sección 3.3 del plan original): confirmado en hardware. Actualmente dibuja dos mitades de pantalla con colores sólidos distintos (rojo tenue para el ojo izquierdo, azul tenue para el derecho) usando dos TRIANGLE_STRIP con shaders mínimos — esto confirma que el mecanismo de split-screen y el contexto EGL funcionan de punta a punta, pero todavía no muestra video real, solo color plano. Sustituir esto por una textura de video real (proveniente de la cámara o de client_core) es trabajo pendiente.
IMU (sección 3.4): confirmado. Se usa ASensorManager_getInstanceForPackage (API correcta para API 26+, no la vieja ASensorManager_getInstance), sensor tipo ROTATION_VECTOR, cuaterniones razonables saliendo cuando se mueve el celular. El mismo cuaternión (x, y, z, w) ahora se empaqueta en 16 bytes (4 floats de 32 bits, little-endian) y se envía por UDP al PC en vez de solo imprimirse por logcat.
Cámara + encoder H.264 (sección 4): confirmado que el encoder de hardware (c2.mtk.avc.encoder, MediaTek) inicia correctamente, produce SPS/PPS válidos (visto como csd-0/csd-1 en el output format del MediaCodec en logcat) y entrega buffers de salida de forma continua. Se detectan 2 cámaras en el dispositivo. Los bindings de la API de cámara del NDK (ACameraManager y compañía) se generaron con bindgen propio contra los headers del NDK, ya que no existe binding maduro listo para esto en el crate ndk.
Bug de permiso de cámara asíncrono: resuelto. requestPermissions de Android es asíncrono (el resultado llega por callback en el hilo de UI, al cual esta arquitectura sin Activity de Java propia no tiene gancho directo), así que se implementó una espera activa (esperar_permiso_camara): revisa cada 500ms si el permiso ya fue concedido, con timeout de 30 segundos, antes de intentar abrir la cámara.
Bug de cierre/ANR al tocar la pantalla: diagnosticado y resuelto por completo. La causa raíz NO era un crash de Rust ni una excepción Java — no hubo ningún FATAL EXCEPTION ni SIGSEGV en ningún logcat. La causa real: android-activity 0.6 requiere que la app confirme (consuma) explícitamente los eventos de input a través de input_events_iter(); como el loop principal no tenía ningún caso para MainEvent::InputAvailable, el evento de touch quedaba sin confirmar ante el sistema, y unos 8 segundos después Android declaraba la app como colgada (ANR) — lo cual generó un volcado de stack vía debuggerd (señal 35) que se veía parecido a un crash pero no lo era. Justo después de eso la Activity se pausaba, y al quedar en segundo plano con el diálogo de ANR encima, el usuario terminó tocando y abriendo otra app (Joplin) sin querer, lo cual generó confusión inicial sobre si la app "se cerraba sola". Fix aplicado: se agregó el caso MainEvent::InputAvailable que itera input_events_iter() y marca cada evento como manejado usando android_activity::InputStatus::Handled (nota de compilación: el enum InputStatus se importa desde la raíz del crate android_activity, NO desde android_activity::input — ese submódulo no lo reexporta públicamente, y esto causó un error de compilación intermedio que ya está resuelto).
Apagado limpio de hilos (mejora no pedida originalmente pero necesaria antes de meter red en vivo): se agregó un Arc<AtomicBool> llamado activo, compartido entre android_main, el hilo de cámara y el hilo de IMU. Al recibir MainEvent::Destroy, se pone en false. El hilo de IMU, que antes bloqueaba indefinidamente en ALooper_pollAll(-1, ...), se cambió a un timeout de 100 milisegundos para poder revisar la bandera periódicamente y salir limpio. El hilo de cámara revisa la misma bandera en cada iteración de su loop de captura de buffers, y al salir hace limpieza explícita: detiene la captura repetitiva, detiene y borra el codec, y cierra la sesión y el dispositivo de cámara.


5. Capa de red implementada (sustituye a la idea original de grabar a archivo local)

El plan original (sección 6, paso 5) proponía primero grabar el H.264 crudo a un archivo local del celular (/data/data/com.pandasz.clientevr/files/camara.h264) y validarlo con ffplay antes de mandar nada por red. Ese paso intermedio se saltó: una vez confirmado en logcat que el encoder producía SPS/PPS y buffers de salida válidos de forma continua, se fue directo a la transmisión UDP en vivo (fusión de los pasos 5 y 6 del plan original). El archivo local ya no se escribe.

Protocolo implementado:


Cámara: cada unidad NAL de salida del encoder se envía como dos datagramas UDP separados y consecutivos al mismo socket: primero 4 bytes little-endian con el tamaño del NAL, luego el NAL crudo. Puerto UDP 5001 en el PC. El socket en el celular se conecta explícitamente (UdpSocket::connect) a la IP del PC configurada.
IMU: cada cuaternión leído se envía como un único datagrama UDP de 16 bytes (4 floats de 32 bits, little-endian: x, y, z, w) al puerto UDP 5002 del PC.
La IP del PC está hardcodeada como constante en src/lib.rs y src/camara.rs (ya reemplazada por el usuario con la IP LAN real de su PC, obtenida vía ipconfig).
Limitación conocida y aceptada por ahora: el framing de "dos datagramas por NAL" no es robusto ante reordenamiento de paquetes UDP (un datagrama podría llegar antes que el otro fuera de orden). En una red Wi-Fi local simple esto es poco probable pero no está garantizado. Si aparecen frames corruptos del lado del receptor Python, este es el primer sospechoso a revisar — la solución sería fusionar tamaño y NAL en un solo send(), o pasar a TCP para esa parte.


Receptor en el PC (receptor_pc.py): dos hilos. Uno escucha el puerto 5002, desempaqueta los 16 bytes del cuaternión y por ahora solo los imprime (integrarlos con la lógica real de tracking del mando es trabajo pendiente). El otro escucha el puerto 5001, reconstruye cada NAL según el protocolo de arriba, lo alimenta por stdin a un proceso ffmpeg corriendo con formato de entrada h264 y salida rawvideo en bgr24, lee los frames crudos por stdout y los muestra en una ventana con OpenCV (cv2.imshow). Requiere ffmpeg accesible en el PATH del sistema — este es el bloqueo activo (ver sección 7).

6. Cosas explícitamente NO hechas todavía (para no asumir que están resueltas)


Sección 3.5 del plan original (conectar con alvr_client_core): no se ha empezado. El repo de ALVR está clonado en C:\proyect-vr-pandasz\ALVR, pero nunca se confirmó en esta conversación haber generado ni inspeccionado el header real alvr_client_core.h. Cualquier nombre de función mencionado en versiones anteriores de la documentación (alvr_initialize, alvr_poll_event, etc.) es un placeholder de ejemplo, no algo verificado contra un header real. Antes de escribir cualquier binding para esto, hay que generar el header de verdad y pegarlo para revisión.
El renderer no muestra video real todavía, solo color sólido de prueba (ver sección 4).
No hay corrección de distorsión de lente en el renderer.
No se ha probado la transmisión UDP en vivo de extremo a extremo con éxito todavía — el bloqueo activo es que ffmpeg no estaba en el PATH del PC al momento de la primera prueba (FileNotFoundError: [WinError 2] al intentar lanzar el subproceso).
No se confirmó si el firewall de Windows permite el tráfico UDP entrante en los puertos 5001 y 5002; se sugirió una regla de netsh advfirewall pero no hay confirmación de que se haya aplicado ni de que haya sido necesaria.
Higiene de hilos JNI: cada función auxiliar que hace vm.attach_current_thread() (permisos, pantalla encendida, ocultar barra de sistema) nunca hace detach explícito. No es un problema actual porque son llamadas puntuales de corta vida, pero si en el futuro alguna de estas se llama repetidamente en un loop, habría que revisar la acumulación de referencias JNI.


7. Siguiente paso inmediato (bloqueo activo al cierre de este documento)

ffmpeg no estaba instalado o no estaba en el PATH del sistema en el PC, causando que receptor_pc.py fallara al intentar lanzar el subproceso. Se indicó instalarlo vía winget (paquete Gyan.FFmpeg) o manualmente desde el build "essentials" de gyan.dev, y abrir una terminal nueva después para que el cambio de PATH tome efecto. El siguiente paso, una vez resuelto esto, es simplemente volver a correr receptor_pc.py y confirmar que aparece la ventana de OpenCV con imagen de la cámara del celular y que el head_quat se sigue imprimiendo en consola de forma continua.

8. Orden sugerido de los siguientes pasos, después de resolver ffmpeg


Confirmar video en vivo funcionando en la ventana de OpenCV, y head_quat imprimiéndose sin cortes, durante al menos un par de minutos seguidos, revisando que no aparezcan errores en adb logcat del lado del celular ni excepciones del lado de Python.
Si aparecen frames corruptos o la imagen se ve rota, revisar primero la limitación de framing UDP descrita en la sección 5.
Generar de verdad el header alvr_client_core.h desde el repo ALVR clonado (sección 3.5 del plan original) y pegarlo para poder escribir los bindings reales, en vez de placeholders.
Reemplazar el renderer de color sólido por una textura de video real.
Integrar la corrección de distorsión de lente.
Integrar el head_quat recibido por UDP con la lógica real de tracking del mando existente del lado de Python (actualmente solo se imprime).