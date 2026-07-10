## ESTADO ACTUAL (actualizado en vivo)

- [x] Esqueleto cargo-apk compilando, instalando y arrancando sin crash.
- [x] Sección 3.3 (renderer EGL/GL split-screen): confirmado en hardware, dos
  mitades de color visibles y pantalla encendida.
- [x] Sección 3.4 (IMU): confirmado, cuaterniones razonables saliendo por logcat
  al mover el celular.
- [x] Sección 4 (cámara + encoder H.264): bindings generados por bindgen propio
  contra el NDK real (no existe binding maduro en el crate `ndk` para esto).
  Encoder inicia correctamente. Detecta 2 cámaras.
- [x] Bug de permisos resuelto: `requestPermissions` es asíncrono, así que se
  agregó una espera activa (`esperar_permiso_camara`, poll cada 500ms, timeout
  30s) antes de intentar abrir la cámara.
- [ ] Pendiente inmediato: confirmar que la cámara abre y graba a
  `/data/data/com.pandasz.clientevr/files/camara.h264` tras conceder el permiso.
- [ ] Bug abierto sin diagnosticar: la app se cierra al tocar la pantalla en el
  Vivo del usuario. Sin stack trace capturado todavía — pendiente de logcat
  limpio del momento exacto del cierre.
- [ ] Sección 5: lado PC en Python (receptor UDP + ffmpeg + head_quat por socket).
- [ ] Sección 6: conectar cámara+IMU con `alvr_client_core` una vez lo anterior
  esté validado end-to-end localmente.

Archivos actuales del proyecto: `Cargo.toml`, `build.rs`, `wrapper.h`,
`src/lib.rs`, `src/renderer.rs`, `src/camara.rs` — el código completo vive en
el repo, no se repite aquí.