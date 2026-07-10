# Módulo de cámara de bajo consumo/latencia dentro de ALVR (Rust/NDK)

## ESTADO ACTUAL (actualizado en vivo)
Progreso real confirmado en tu máquina, en orden:
- [x] NDK instalado (`27.1.12297006`) vía Android Studio.
- [x] Rust instalado (`cargo 1.97.0`) vía rustup.
- [x] Targets Android agregados (`aarch64`, `armv7`, `x86_64`, `i686`).
- [x] `cargo-ndk` y `cargo-apk` instalados.
- [x] Variables de entorno permanentes (`ANDROID_HOME`, `ANDROID_NDK_HOME`, `JAVA_HOME`) con `setx`.
- [x] Repo ALVR clonado en `C:\proyect-vr-pandasz\ALVR`.
- [x] `cargo xtask prepare-deps --platform android` corrido con éxito (nota: el flag correcto
      en esta versión de ALVR es `--platform android`, **no** `--android` como decía la v1
      de este plan).
- [x] Submódulo `openvr` inicializado (`git submodule update --init --recursive`) — necesario
      porque `git clone` normal no trae submódulos, y `alvr_session` los necesita para compilar.
- [x] `cargo xtask build-client-lib` corrido con éxito. Este es el subcomando real (confirmado
      contra `cargo xtask --help`), no había que adivinarlo.
- [x] Header real `alvr_client_core.h` generado e inspeccionado (ver sección 1.1 más abajo,
      reemplaza a la "Nota de honestidad" original — ya no aplica, el header es conocido).
- [x] Proyecto propio creado: `C:\proyect-vr-pandasz\cliente_vr_pandasz` (nota: el nombre final
      del proyecto es `cliente_vr_pandasz`, no `mi_cliente_vr` como en el borrador original de
      este documento — ajusta cualquier referencia a rutas si copias comandos de abajo).
- [ ] Configurar `Cargo.toml` del proyecto (siguiente paso).
- [ ] Resto de la sección 3 en adelante: pendiente.


## 0. Confirmación de licencia
El cliente de ALVR está licenciado en **MIT**. Extenderlo con tu propio módulo, sin tocar
ni copiar código de PhoneVR (GPL), mantiene tu trabajo en MIT sin problema. La única
regla real: no copiar/pegar código de PhoneVR — usarlo solo como referencia conceptual
de "qué hace", no de "cómo está escrito", como ya tenías pensado.

## 1. Corrección de arquitectura (importante, v2)

**Corrección sobre la corrección:** en la primera versión de este documento asumí
que podíamos partir del cliente oficial `alvr_client_openxr` de ALVR. Eso estaba
mal para tu caso: ese cliente requiere un **runtime OpenXR**, que solo traen los
cascos standalone reales (Quest, Pico, etc.) — **un celular normal no lo tiene**,
así que ese APK ni siquiera arrancaría como visor en tu teléfono.

Lo que de verdad corre en tu celular es **PhoneVR**, un proyecto totalmente
distinto: C++ nativo con JNI, renderiza en estilo Google Cardboard (dos mitades
de pantalla, sin OpenXR) — por eso sí funciona en un celular cualquiera. Y es
**GPL-3.0**. Con tu restricción de mantener licencia permisiva, tocar o extender
el código de PhoneVR queda descartado.

**La pieza que sí resuelve esto limpio:** dentro del propio repo de ALVR (MIT)
existe `alvr/client_core/` — descrito por ellos mismos como *"código de cliente
agnóstico de plataforma... también compilable a librería C ABI con un .h para
integración con otros proyectos"*. Es decir, ALVR ya separó a propósito toda la
lógica de red/protocolo/decodificación de video (`client_core`) de su renderer
OpenXR (`client_openxr`), exactamente para que terceros construyan clientes
alternativos. Esa es la base correcta:

- `alvr_client_core` (MIT) → todo el trabajo de red, descubrimiento, protocolo y
  decodificación de video por hardware. **No la tocamos, la consumimos tal cual.**
- Tu propio renderer Cardboard-style (OpenGL ES, split-screen) → nuevo, tuyo, MIT.
- Tu módulo de cámara del mando (sección de más abajo) → nuevo, tuyo, MIT.

Todo dentro de una sola APK nueva que tú controlas por completo, sin una sola
línea de PhoneVR ni de `client_openxr`.

### Nota de honestidad sobre el header de `client_core`
No tengo el listado exacto y verificado de todas las funciones que expone
`alvr_client_core.h` (no pude inspeccionar el header generado desde aquí, ya que
no tengo el toolchain de Android/NDK en este entorno para compilarlo). El plan de
abajo te dice exactamente cómo generarlo tú y qué patrón típico esperar (init /
poll de eventos / entrega de frame decodificado), pero **la firma exacta de cada
función la confirmas contra el header real una vez lo generes** en el paso 3.1.
No quiero darte funciones inventadas presentadas como si ya las hubiera verificado.

## 2. Instalación desde cero (Android Studio sin NDK, Rust sin cargo-ndk)

Vamos paso a paso, en orden, para tu caso exacto.

### 2.1 Instalar el NDK (usando el Android Studio que ya tienes)
1. Abre Android Studio sin ningún proyecto abierto.
2. `More Actions` (o `File > Settings` si ya tienes un proyecto abierto) →
   `Languages & Frameworks` → `Android SDK` → pestaña **SDK Tools**.
3. Marca la casilla **NDK (Side by side)** y **CMake** → Apply → espera la descarga.
4. Anota la ruta que te muestra arriba de la lista (algo como
   `C:\Users\<tu_usuario>\AppData\Local\Android\Sdk`) — es tu `ANDROID_HOME`.

Esto instala la versión más reciente del NDK. **ALVR fija una versión específica**
del NDK en su CI (a la fecha de este documento, `25.1.8937393` — puede cambiar).
Si el build falla más adelante con errores de NDK, instala esa versión exacta
además de la que ya tienes, desde la misma pestaña SDK Tools → botón
"Show Package Details" (arriba a la derecha) → marca la versión específica.

### 2.2 Targets de Rust para Android
En una terminal (con tu Rust ya instalado):
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
```
(los cuatro, aunque tu celular real seguramente solo necesita `aarch64-linux-android`
— los demás son por si compilas para emulador x86_64 o dispositivos ARMv7 viejos)

### 2.3 Herramientas de cargo que te faltan
```bash
cargo install cargo-ndk
cargo install cargo-apk
```

### 2.4 Variables de entorno (Windows — PowerShell)
Para la sesión actual de PowerShell:
```powershell
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_NDK_HOME = "$env:ANDROID_HOME\ndk\25.1.8937393"
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
```
Ojo con `JAVA_HOME`: en versiones más nuevas de Android Studio la carpeta del JDK
embebido se llama `jbr` (JetBrains Runtime); en versiones viejas era `jre`. Revisa
cuál de las dos existe realmente dentro de tu carpeta de instalación de Android
Studio y usa esa.

Para que quede permanente (no solo en esta sesión de PowerShell), usa
`setx ANDROID_HOME "..."`, `setx ANDROID_NDK_HOME "..."`, `setx JAVA_HOME "..."`
y **abre una terminal nueva** después (setx no afecta la terminal actual).

### 2.5 Clonar ALVR y compilar SOLO `client_core` (no el cliente OpenXR completo)
```bash
git clone https://github.com/alvr-org/ALVR.git
cd ALVR
cargo xtask prepare-deps --android
cargo xtask --help
```
Ese último comando (`--help`) es a propósito: la documentación pública confirma
que existe un paso de build específico para `client_core` (separado del build
completo del cliente OpenXR), pero no tengo el nombre exacto del subcomando
verificado desde aquí. Búscalo en la salida de `--help` (algo con "client-core"
o "client-lib" en el nombre) y ejecútalo. El resultado esperado, según su propio
sistema de build, es:
```
build/alvr_client_core/
├── arm64-v8a/libalvr_client_core.so
└── alvr_client_core.h        <-- este header es tu contrato real con la librería
```
**Abre ese `.h` con cualquier editor** y anota las funciones que expone (típicamente
algo como `alvr_init`, `alvr_poll_event`, funciones para entregar el tamaño de
video/entorno de decodificación, y para enviar tracking). Ese archivo es la
fuente de verdad — más confiable que cualquier ejemplo de código que te dé aquí
sin haberlo visto yo mismo.

## 3. Proyecto nuevo: tu propio cliente Cardboard + cámara (todo MIT)

### 3.1 Crear el esqueleto del proyecto con cargo-apk
```bash
cargo new --lib mi_cliente_vr
cd mi_cliente_vr
```
`Cargo.toml`:
```toml
[package]
name = "mi_cliente_vr"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
android-activity = { version = "0.6", features = ["native-activity"] }
ndk = "0.9"
ndk-sys = "0.6"
log = "0.4"
android_logger = "0.14"

[package.metadata.android]
package = "com.tuusuario.miclientevr"
build_targets = ["aarch64-linux-android"]

[package.metadata.android.sdk]
min_sdk_version = 26
target_sdk_version = 33

[[package.metadata.android.uses_feature]]
name = "android.hardware.camera"
required = true
[[package.metadata.android.uses_permission]]
name = "android.permission.CAMERA"
[[package.metadata.android.uses_permission]]
name = "android.permission.INTERNET"
```
(Las versiones exactas de `android-activity`/`ndk` pueden haber subido desde que
escribí esto — `cargo add android-activity ndk ndk-sys` te resuelve la última.)

### 3.2 Punto de entrada (`src/lib.rs`)
```rust
use android_activity::{AndroidApp, MainEvent, PollEvent};
use log::info;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_tag("MiClienteVR"));
    info!("Arrancando mi cliente VR");

    // Hilo del render Cardboard (seccion 3.3) -- necesita la ventana nativa
    // que entrega `app`, asi que se coordina con los eventos Resumed/Destroyed.

    // Hilo del sensor IMU / cabeza (seccion 3.4)
    std::thread::spawn(|| leer_sensores_cabeza());

    // Hilo de camara + encoder + UDP (seccion 4, ya diseñado antes)
    std::thread::spawn(|| modulo_camara_udp());

    // TODO seccion 3.5: inicializar alvr_client_core aqui, usando las
    // funciones reales que confirmes en el header (3.1).

    let mut quit = false;
    while !quit {
        app.poll_events(Some(std::time::Duration::from_millis(16)), |event| {
            match event {
                PollEvent::Main(MainEvent::Destroy) => quit = true,
                _ => {}
            }
        });
    }
}
```

### 3.3 Renderer Cardboard-style (esqueleto OpenGL ES)
La idea mínima viable: una superficie con `glViewport` dividido en dos mitades
(ojo izq/der), cada mitad dibuja la MISMA textura de video (la que decodifica
`client_core`) con un desplazamiento horizontal de cámara distinto por ojo
(estéreo simple). Sin corrección de distorsión de lente en la v1 — eso se agrega
después una vez que ya ves imagen en las dos mitades:

```rust
// pseudo-flujo, usando ndk::native_window para el ANativeWindow y
// EGL a mano (o el crate `khronos-egl`) para el contexto GL:
//
// 1. egl::create_context() sobre la ventana nativa que entrega android-activity
// 2. glViewport(0, 0, ancho/2, alto)          -> ojo izquierdo
//    dibujar quad con la textura de video, UV desplazadas -ipd/2
// 3. glViewport(ancho/2, 0, ancho/2, alto)     -> ojo derecho
//    mismo quad, UV desplazadas +ipd/2
// 4. eglSwapBuffers()
```
Esto es standard OpenGL ES de "split screen estéreo" — mismo principio que
cualquier tutorial de Cardboard, solo que en Rust con bindings crudos en vez de
Java. Te lo puedo desarrollar línea por línea en el siguiente mensaje si quieres
(es su propia sección grande, mejor no mezclarla con todo lo demás de una vez).

### 3.4 Orientación de cabeza (IMU, sin OpenXR)
Como no hay OpenXR, la rotación de cabeza sale directo de los sensores de
Android vía NDK (`ASensorManager`, disponible sin JNI):
```rust
use ndk_sys::{ASensorManager_getInstance, ASensorManager_getDefaultSensor,
              ASENSOR_TYPE_ROTATION_VECTOR};

fn leer_sensores_cabeza() {
    unsafe {
        let mgr = ASensorManager_getInstance(); // API vieja; en API 26+
                                                 // usar ASensorManager_getInstanceForPackage
        let sensor = ASensorManager_getDefaultSensor(mgr, ASENSOR_TYPE_ROTATION_VECTOR as i32);
        // crear ASensorEventQueue, activarlo, loop leyendo eventos ->
        // el evento de tipo ROTATION_VECTOR ya te da un cuaternion
        // (x,y,z,w) directo -- exactamente lo que tu script de Python
        // espera para "hq" (head_quat). Cero matematica extra necesaria.
    }
}
```
Este es el mismo cuaternión que ya usas en Python como `head_quat` — así que del
lado del PC no cambia nada de la lógica de tracking del mando, solo cambia CÓMO
llega ese cuaternión (antes vía OpenVR/casco, ahora directo del sensor del
celular por tu propio socket).

### 3.5 Conectar con `alvr_client_core`
Aquí es donde entra el header que generaste en 2.5. El patrón típico de este
tipo de librerías (init → poll de eventos → entrega de frame) suele verse así,
**pero confirma cada nombre contra tu `alvr_client_core.h` real**:
```rust
extern "C" {
    // nombres de ejemplo -- reemplaza por los reales del header
    fn alvr_initialize(/* ... */);
    fn alvr_poll_event(/* ... */) -> bool;
    fn alvr_send_tracking(/* head pose, controller pose */);
}
```
Te recomiendo hacer esta parte en un paso aparte, una vez tengas el header en
mano — dámelo (o pégame las firmas) y te armo el binding real, en vez de
adivinar.

## 4. Módulo de cámara del mando (igual que se diseñó antes)
Vive en el mismo proyecto (`mi_cliente_vr`), como otro hilo independiente:

| Necesidad | API NDK | Soporte en Rust |
|---|---|---|
| Capturar cámara | `ACameraManager`/`ACameraDevice`/`ACameraCaptureSession`/`AImageReader` | Sin binding maduro — bindgen propio (ver ejemplo `drkstr101/CartoonifyIt`) |
| Codificar H.264 hw | `AMediaCodec` | Ya soportado por el crate `ndk` (`ndk::media_codec`) |
| Enviar al PC | UDP crudo | `std::net::UdpSocket` |

```rust
// build.rs: bindgen contra camera/NdkCameraManager.h, NdkCameraDevice.h,
// NdkCameraCaptureSession.h, NdkCameraMetadataTags.h, media/NdkImageReader.h
// enlazando libcamera2ndk.so y libmediandk.so (API 24+).
```
```rust
use ndk::media_codec::{MediaCodec, MediaFormat};

fn modulo_camara_udp() {
    let format = MediaFormat::new();
    format.set_str("mime", "video/avc");
    format.set_i32("width", 640);
    format.set_i32("height", 480);
    format.set_i32("bitrate", 4_000_000);
    format.set_i32("frame-rate", 60);
    format.set_i32("color-format", 0x7f000789); // COLOR_FormatSurface

    let codec = MediaCodec::from_encoder_type("video/avc").unwrap();
    codec.configure(&format, /* flags: ENCODE */).unwrap();
    let _input_surface = codec.create_input_surface().unwrap(); // <- camara conecta aqui
    codec.start().unwrap();

    let sock = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    sock.connect(("192.168.1.50", 5001)).unwrap(); // IP de tu PC, puerto nuevo
    loop {
        // por cada NAL de salida del codec:
        // let len = (nal.len() as u32).to_le_bytes();
        // sock.send(&len).unwrap();
        // sock.send(&nal).unwrap();
    }
}
```

## 5. Lado PC (Python)
Reemplazar `cv2.VideoCapture(IPCAM_URL)` por un receptor que:
1. Escuche el socket UDP del puerto 5001 (los mismos `[4 bytes tamaño][NAL]`).
2. Alimente un proceso `ffmpeg` por stdin con esos NAL crudos, y lea frames BGR
   crudos por stdout con `np.frombuffer` para seguir usando OpenCV igual que hoy.
   (Evita reinventar un decodificador H.264 en Python puro.)
3. La cabeza (`head_quat`) ya no viene de OpenVR — llega por tu propio socket
   desde el hilo de la sección 3.4 (mismo formato de cuaternión que ya usas).

## 6. Plan de validación por etapas (no lo hagas todo de un tirón)
1. **Build limpio de `client_core`** (sección 2.5) y revisar su header real.
2. **Esqueleto `cargo apk build`**: el proyecto de la sección 3.1-3.2 debe
   compilar e instalar, aunque solo muestre pantalla negra con un log en
   `adb logcat` — confirma que tu entorno cargo-apk/NDK está bien antes de
   sumar cámara, IMU o client_core.
3. **IMU**: agrega la sección 3.4, confirma por logcat que salen cuaterniones
   razonables al mover el celular.
4. **Renderer**: agrega el split-screen de la sección 3.3, sin distorsión de
   lente todavía — solo confirma que ves algo en las dos mitades.
5. **Cámara + encoder local**: agrega la sección 4, guarda el H.264 en un
   archivo del celular y revísalo con `ffplay` antes de mandarlo por red.
6. **Red**: conecta el UDP de cámara y de IMU con tu PC.
7. **`client_core`**: la parte más incierta — una vez tengas 2-6 funcionando,
   integras el video real de SteamVR con las funciones reales del header.

## Nota de honestidad
No tengo forma de compilar ni correr nada de esto contra un dispositivo real
desde aquí (no tengo SDK/NDK de Android ni celular conectado en este entorno).
Todo el código de este documento es un **primer boceto de arquitectura**, no
código verificado en hardware. Dos puntos concretos de mayor incertidumbre:
los bindings de cámara NDK (poca documentación/comunidad, espera iterar) y la
integración real con `alvr_client_core` (sus nombres de función exactos los
confirmas tú contra el header, no yo). Todo lo demás (IMU, UDP, estructura del
proyecto cargo-apk) es más estándar y tiene mayor probabilidad de funcionar
tal cual está escrito aquí, en el primer intento.