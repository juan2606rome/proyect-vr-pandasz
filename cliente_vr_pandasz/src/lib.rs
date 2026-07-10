mod renderer;
mod camara;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use log::info;
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
    pedir_permiso_camara(&app);

    std::thread::spawn(|| leer_sensores_cabeza());

    let app_camara = app.clone();
    std::thread::spawn(move || camara::iniciar_camara(&app_camara));

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
        let package_name = std::ffi::CString::new("com.pandasz.clientevr").unwrap();
        let mgr = ndk_sys::ASensorManager_getInstanceForPackage(package_name.as_ptr());
        if mgr.is_null() {
            log::error!("No se pudo obtener ASensorManager");
            return;
        }
        let sensor = ndk_sys::ASensorManager_getDefaultSensor(
            mgr,
            ndk_sys::ASENSOR_TYPE_ROTATION_VECTOR as i32,
        );
        if sensor.is_null() {
            log::error!("Este dispositivo no tiene sensor ROTATION_VECTOR");
            return;
        }
        let ident = 1;
        let queue = ndk_sys::ASensorManager_createEventQueue(
            mgr, looper, ident, None, std::ptr::null_mut(),
        );
        ndk_sys::ASensorEventQueue_enableSensor(queue, sensor);
        ndk_sys::ASensorEventQueue_setEventRate(queue, sensor, 1_000_000 / 60);
        info!("Sensor IMU inicializado, esperando eventos de rotación...");
        loop {
            let ret = ndk_sys::ALooper_pollAll(-1, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            if ret == ident {
                let mut events: [ndk_sys::ASensorEvent; 8] = std::mem::zeroed();
                let count = ndk_sys::ASensorEventQueue_getEvents(queue, events.as_mut_ptr(), events.len());
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

/// Consulta directa a `ContextCompat.checkSelfPermission`. Público a nivel de
/// crate para que `camara.rs` pueda esperar a que el usuario acepte el diálogo.
pub fn permiso_camara_concedido(app: &AndroidApp) -> bool {
    unsafe {
        let vm = jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _).unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = jni::objects::JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject);
        let perm_str = env.new_string("android.permission.CAMERA").unwrap();
        let permiso = env
            .call_method(&activity, "checkSelfPermission", "(Ljava/lang/String;)I",
                &[jni::objects::JValue::Object(&perm_str)])
            .unwrap().i().unwrap();
        permiso == 0
    }
}

fn pedir_permiso_camara(app: &AndroidApp) {
    unsafe {
        if permiso_camara_concedido(app) {
            info!("Permiso de cámara ya concedido");
            return;
        }
        let vm = jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _).unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = jni::objects::JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject);
        info!("Pidiendo permiso de cámara al usuario...");
        let perm_str = env.new_string("android.permission.CAMERA").unwrap();
        let perm_array = env.new_object_array(1, "java/lang/String", &perm_str).unwrap();
        env.call_method(&activity, "requestPermissions", "([Ljava/lang/String;I)V",
            &[jni::objects::JValue::Object(&perm_array), jni::objects::JValue::Int(1001)])
            .unwrap();
    }
}

fn mantener_pantalla_encendida(app: &AndroidApp) {
    unsafe {
        let vm = jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _).unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = jni::objects::JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject);
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l().unwrap();
        env.call_method(&window, "addFlags", "(I)V", &[jni::objects::JValue::Int(0x00000080)]).unwrap();
    }
}

fn ocultar_barra_sistema(app: &AndroidApp) {
    unsafe {
        let vm = jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _).unwrap();
        let mut env = vm.attach_current_thread().unwrap();
        let activity = jni::objects::JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject);
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l().unwrap();
        let decor_view = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[]).unwrap().l().unwrap();
        env.call_method(&decor_view, "setSystemUiVisibility", "(I)V", &[jni::objects::JValue::Int(5894)]).unwrap();
    }
}