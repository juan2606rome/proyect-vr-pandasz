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