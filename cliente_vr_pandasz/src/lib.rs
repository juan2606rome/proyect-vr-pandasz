use android_activity::{AndroidApp, MainEvent, PollEvent};
use log::{info, error};
use std::ffi::CString;

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
    unsafe {
        ndk_sys::ALooper_prepare(ndk_sys::ALOOPER_PREPARE_ALLOW_NON_CALLBACKS as i32);
        let looper = ndk_sys::ALooper_forThread();

        let package_name = CString::new("com.pandasz.clientevr").unwrap();
        let mgr = ndk_sys::ASensorManager_getInstanceForPackage(package_name.as_ptr());
        if mgr.is_null() {
            error!("No se pudo obtener ASensorManager");
            return;
        }

        let sensor = ndk_sys::ASensorManager_getDefaultSensor(
            mgr,
            ndk_sys::ASENSOR_TYPE_ROTATION_VECTOR as i32,
        );
        if sensor.is_null() {
            error!("Este dispositivo no tiene sensor ROTATION_VECTOR");
            return;
        }

        let ident = 1;
        let queue = ndk_sys::ASensorManager_createEventQueue(
            mgr,
            looper,
            ident,
            None,
            std::ptr::null_mut(),
        );
        ndk_sys::ASensorEventQueue_enableSensor(queue, sensor);
        ndk_sys::ASensorEventQueue_setEventRate(queue, sensor, 1_000_000 / 60);

        info!("Sensor IMU inicializado, esperando eventos de rotación...");

        loop {
            let ret = ndk_sys::ALooper_pollAll(
                -1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if ret == ident {
                let mut events: [ndk_sys::ASensorEvent; 8] = std::mem::zeroed();
                let count = ndk_sys::ASensorEventQueue_getEvents(
                    queue,
                    events.as_mut_ptr(),
                    events.len(),
                );
                for i in 0..count as usize {
                    let ev = &events[i];
                    let data = ev.__bindgen_anon_1.__bindgen_anon_1.data;
                    let (x, y, z, w) = (data[0], data[1], data[2], data[3]);
                    info!("head_quat x={:.4} y={:.4} z={:.4} w={:.4}", x, y, z, w);
                }
            }
        }
    }
}

fn modulo_camara_udp() {
    // sección 4 — placeholder por ahora para que compile
}