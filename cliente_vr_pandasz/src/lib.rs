mod renderer;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use log::{info, error};
use std::ffi::CString;
use renderer::Renderer;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("ClienteVrPandasz")
            .with_max_level(log::LevelFilter::Trace),
    );
    info!("Arrancando cliente_vr_pandasz");
    mantener_pantalla_encendida(&app);
    ocultar_barra_sistema(&app);


    std::thread::spawn(|| leer_sensores_cabeza());
    // std::thread::spawn(|| modulo_camara_udp()); // sección 4, todavía no

    let mut renderer: Option<Renderer> = None;
    let mut quit = false;

    while !quit {
        app.poll_events(Some(std::time::Duration::from_millis(16)), |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    if let Some(window) = app.native_window() {
                        renderer = Renderer::new(&window);
                    }
                }
                PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                    renderer = None;
                }
                PollEvent::Main(MainEvent::Destroy) => quit = true,
                _ => {}
            }
        });

        if let Some(r) = &renderer {
            r.dibujar_frame();
        }
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


fn mantener_pantalla_encendida(app: &AndroidApp) {
    unsafe {
        let vm_ptr = app.vm_as_ptr();
        let activity_ptr = app.activity_as_ptr();
        let vm = jni::JavaVM::from_raw(vm_ptr as *mut _).unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);

        let window = env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .unwrap()
            .l()
            .unwrap();

        // FLAG_KEEP_SCREEN_ON = 0x00000080
        env.call_method(&window, "addFlags", "(I)V", &[jni::objects::JValue::Int(0x00000080)])
            .unwrap();
    }
}

fn ocultar_barra_sistema(app: &AndroidApp) {
    unsafe {
        let vm_ptr = app.vm_as_ptr();
        let activity_ptr = app.activity_as_ptr();
        let vm = jni::JavaVM::from_raw(vm_ptr as *mut _).unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);

        let window = env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .unwrap()
            .l()
            .unwrap();

        let decor_view = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .unwrap()
            .l()
            .unwrap();

        // Combinación de flags clásicos "immersive sticky":
        // LAYOUT_STABLE | LAYOUT_HIDE_NAVIGATION | LAYOUT_FULLSCREEN |
        // HIDE_NAVIGATION | FULLSCREEN | IMMERSIVE_STICKY = 5894
        env.call_method(
            &decor_view,
            "setSystemUiVisibility",
            "(I)V",
            &[jni::objects::JValue::Int(5894)],
        )
        .unwrap();
    }
}

#[allow(dead_code)]
fn modulo_camara_udp() {
    // sección 4 — todavía no implementado
}

