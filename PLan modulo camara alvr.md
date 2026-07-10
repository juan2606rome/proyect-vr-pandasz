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
- [x] `Cargo.toml` configurado: `name = "cliente_vr_pandasz"`, `package = "com.pandasz.clientevr"`,
  dependencias (`android-activity`, `ndk`, `ndk-sys`, `log`, `android_logger`) agregadas y
  resueltas con `cargo add`.
- [x] `cargo ndk -t arm64-v8a check` corrido con éxito — confirma que el toolchain Android
  completo (NDK + cargo-ndk + targets) funciona de punta a punta. (Nota: `cargo check` a
  secas SIEMPRE va a fallar en este proyecto porque usa el target de la máquina host en vez
  del target Android — para verificar el código hay que usar siempre `cargo ndk ... check`,
  no `cargo check`.)
- [x] `src/lib.rs` con el esqueleto de `android_main` escrito: dos hilos placeholder
  (`leer_sensores_cabeza` para sección 3.4 e IMU, `modulo_camara_udp` para sección 4 y
  cámara/UDP), loop de eventos con salida en `MainEvent::Destroy`.
- [x] `cargo apk build` corrido con éxito, APK instalada en el celular con `adb install`.
- [x] App confirmada arrancando sin crash: `ActivityTaskManager: Displayed
  com.pandasz.clientevr/android.app.NativeActivity` en logcat, sin `FATAL EXCEPTION` ni stack
  trace de `AndroidRuntime`. Pantalla negra es el resultado esperado en este punto (todavía no
  hay renderer, eso es la sección 3.3, pendiente).
- [x] Diagnosticado por qué el log propio (`info!("Arrancando cliente_vr_pandasz")`) no
  aparecía en `adb logcat -s ClienteVrPandasz:V`: `android_logger::Config::default()` no fija
  nivel máximo, así que filtraba las llamadas a `info!()`. Corrección aplicada en `src/lib.rs`:
  ```rust
  android_logger::init_once(
      android_logger::Config::default()
          .with_max_level(log::LevelFilter::Trace)
          .with_tag("ClienteVrPandasz"),
  );
  ```
  Rebuild + reinstalación (`adb uninstall` seguido de `cargo apk build` + `adb install`) hechos hoy.
- [ ] **Pendiente inmediato**: confirmar con `adb logcat -d -s ClienteVrPandasz:V` que ya
  aparece la línea `Arrancando cliente_vr_pandasz` tras el fix del logger, y confirmar con
  `adb shell ps | findstr pandasz` que el proceso sigue vivo unos segundos después de abrir la
  app (para descartar un crash silencioso sin traza en logcat).
- [ ] Sección 3.4: IMU / orientación de cabeza (`ASensorManager`, cuaternión de rotación).
- [ ] Sección 3.3: Renderer Cardboard-style (OpenGL ES, split-screen).
- [ ] Sección 3.5: conectar con `alvr_client_core` usando las funciones reales del header.
- [ ] Sección 4: módulo de cámara + encoder H.264 + UDP.
- [ ] Sección 5: lado PC en Python (receptor UDP + ffmpeg + head_quat por socket propio).

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
de pantalla, sin OpenXR) — por eso sí funciona en un celular cualquiera. Y es **GPL-3.0**. Con tu restricción de mantener licencia permisiva, tocar o extender
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

No tengo el listado exacto y verificado de todas las funciones que expone `alvr_client_core.h` (no pude inspeccionar el header generado desde aquí, ya que
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

Esto instala la versión más reciente del NDK. **ALVR fija una versión específica** del NDK en su CI (a la fecha de este documento, `25.1.8937393` — puede cambiar).
Si el build falla más adelante con errores de NDK, instala esa versión exacta
además de la que ya tienes, desde la misma pestaña SDK Tools → botón
"Show Package Details" (arriba a la derecha) → marca la versión específica.

### 2.2 Targets de Rust para Android

En una terminal (con tu Rust ya instalado):

```
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
```

(los cuatro, aunque tu celular real seguramente solo necesita `aarch64-linux-android` — los demás son por si compilas para emulador x86_64 o dispositivos ARMv7 viejos)

### 2.3 Herramientas de cargo que te faltan

```
cargo install cargo-ndk
cargo install cargo-apk
```

### 2.4 Variables de entorno (Windows — PowerShell)

Para la sesión actual de PowerShell:

```
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_NDK_HOME = "$env:ANDROID_HOME\ndk\25.1.8937393"
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
```

Ojo con `JAVA_HOME`: en versiones más nuevas de Android Studio la carpeta del JDK
embebido se llama `jbr` (JetBrains Runtime); en versiones viejas era `jre`. Revisa
cuál de las dos existe realmente dentro de tu carpeta de instalación de Android
Studio y usa esa.

Para que quede permanente (no solo en esta sesión de PowerShell), usa `setx ANDROID_HOME "..."`, `setx ANDROID_NDK_HOME "..."`, `setx JAVA_HOME "..."` y **abre una terminal nueva** después (setx no afecta la terminal actual).

### 2.5 Clonar ALVR y compilar SOLO `client_core` (no el cliente OpenXR completo)

```
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

```
cargo new --lib cliente_vr_pandasz
cd cliente_vr_pandasz
```

`Cargo.toml` (versión real usada, con el nombre y paquete definitivos):

```
[package]
name = "cliente_vr_pandasz"
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
package = "com.pandasz.clientevr"
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

Versión real usada, ya con el fix de logging (`with_max_level`) incorporado:

```rust
use android_activity::{AndroidApp, MainEvent, PollEvent};
use log::info;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag("ClienteVrPandasz"),
    );
    info!("Arrancando cliente_vr_pandasz");

    // Hilo del sensor IMU / cabeza (sección 3.4, la agregamos después)
    std::thread::spawn(|| leer_sensores_cabeza());

    // Hilo de cámara + encoder + UDP (sección 4, la agregamos después)
    std::thread::spawn(|| modulo_camara_udp());

    // TODO sección 3.5: inicializar alvr_client_core aquí

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

fn leer_sensores_cabeza() {
    // sección 3.4 — placeholder por ahora para que compile
}

fn modulo_camara_udp() {
    // sección 4 — placeholder por ahora para que compile
}
```

### 3.3 Renderer Cardboard-style (esqueleto OpenGL ES)

La idea mínima viable: una superficie con `glViewport` dividido en dos mitades
(ojo izq/der), cada mitad dibuja la MISMA textura de video (la que decodifica `client_core`) con un desplazamiento horizontal de cámara distinto por ojo
(estéreo simple). Sin corrección de distorsión de lente en la v1 — eso se agrega
después una vez que ya ves imagen en las dos mitades:

```
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
tipo de librerías (init → poll de eventos → entrega de frame) suele verse así, **pero confirma cada nombre contra tu `alvr_client_core.h` real**:

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

Vive en el mismo proyecto (`cliente_vr_pandasz`), como otro hilo independiente:

| Necesidad          | API NDK                                                                 | Soporte en Rust                                                            |
| ------------------ | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| Capturar cámara    | `ACameraManager`/`ACameraDevice`/`ACameraCaptureSession`/`AImageReader` | Sin binding maduro — bindgen propio (ver ejemplo `drkstr101/CartoonifyIt`) |
| Codificar H.264 hw | `AMediaCodec`                                                           | Ya soportado por el crate `ndk` (`ndk::media_codec`)                       |
| Enviar al PC       | UDP crudo                                                               | `std::net::UdpSocket`                                                      |

```
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
   sumar cámara, IMU o client_core. **✅ Hecho hoy — APK compila, instala y arranca
   sin crash. Pendiente solo confirmar que el log propio ya es visible tras el
   fix del logger.**
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
código verificado en hardware más allá de lo que tú mismo has confirmado y
pegado en la conversación (Cargo.toml, esqueleto de lib.rs, y el build/instalación
en tu celular real). Dos puntos concretos de mayor incertidumbre siguen siendo:
los bindings de cámara NDK (poca documentación/comunidad, espera iterar) y la
integración real con `alvr_client_core` (sus nombres de función exactos los
confirmas tú contra el header, no yo). Todo lo demás (IMU, UDP, estructura del
proyecto cargo-apk) es más estándar y tiene mayor probabilidad de funcionar
tal cual está escrito aquí, en el primer intento.
