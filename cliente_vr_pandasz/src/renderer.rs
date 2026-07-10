use glow::HasContext;
use khronos_egl as egl;
use ndk::native_window::NativeWindow;
use log::{info, error};

pub struct Renderer {
    egl: egl::Instance<egl::Dynamic<libloading::Library, egl::EGL1_4>>,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    gl: glow::Context,
    program: glow::Program,
    vao_left: glow::VertexArray,
    vao_right: glow::VertexArray,
    ancho: i32,
    alto: i32,
}

impl Renderer {
    pub fn new(window: &NativeWindow) -> Option<Self> {
        unsafe {
            let lib = libloading::Library::new("libEGL.so").ok()?;
            let egl = egl::Instance::new(egl::Dynamic::<libloading::Library, egl::EGL1_4>::load_required(lib).ok()?);

            let display = egl.get_display(egl::DEFAULT_DISPLAY)?;
            egl.initialize(display).ok()?;

            let attribs = [
                egl::RED_SIZE, 8,
                egl::GREEN_SIZE, 8,
                egl::BLUE_SIZE, 8,
                egl::SURFACE_TYPE, egl::WINDOW_BIT,
                egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
                egl::NONE,
            ];
            let config = egl.choose_first_config(display, &attribs).ok()?.expect("sin config EGL compatible");

            let context_attribs = [egl::CONTEXT_CLIENT_VERSION, 2, egl::NONE];
            let context = egl.create_context(display, config, None, &context_attribs).ok()?;

            let native_window_ptr = window.ptr().as_ptr() as egl::NativeWindowType;
            let surface = egl.create_window_surface(display, config, native_window_ptr, None).ok()?;

            egl.make_current(display, Some(surface), Some(surface), Some(context)).ok()?;

            let gl = glow::Context::from_loader_function(|s| {
                egl.get_proc_address(s).map(|p| p as *const _).unwrap_or(std::ptr::null())
            });

            let ancho = window.width();
            let alto = window.height();

            let (program, vao_left, vao_right) = Self::crear_recursos_gl(&gl);

            info!("Renderer EGL inicializado: {}x{}", ancho, alto);

            Some(Self { egl, display, context, surface, gl, program, vao_left, vao_right, ancho, alto })
        }
    }

    // Ya es `unsafe fn`, así que todo el cuerpo puede llamar funciones GL
    // sin necesitar otro bloque `unsafe { }` adentro (por eso salía el warning).
    unsafe fn crear_recursos_gl(gl: &glow::Context) -> (glow::Program, glow::VertexArray, glow::VertexArray) {
        let vertex_src = r#"#version 100
            attribute vec2 pos;
            attribute vec2 uv;
            varying vec2 v_uv;
            void main() {
                v_uv = uv;
                gl_Position = vec4(pos, 0.0, 1.0);
            }
        "#;
        let fragment_src = r#"#version 100
            precision mediump float;
            varying vec2 v_uv;
            uniform vec3 color_ojo;
            void main() {
                gl_FragColor = vec4(color_ojo * (0.5 + 0.5 * v_uv.x), 1.0);
            }
        "#;

        let program = gl.create_program().unwrap();
        let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
        gl.shader_source(vs, vertex_src);
        gl.compile_shader(vs);
        if !gl.get_shader_compile_status(vs) {
            error!("Error compilando vertex shader: {}", gl.get_shader_info_log(vs));
        }
        let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
        gl.shader_source(fs, fragment_src);
        gl.compile_shader(fs);
        if !gl.get_shader_compile_status(fs) {
            error!("Error compilando fragment shader: {}", gl.get_shader_info_log(fs));
        }
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);

        gl.bind_attrib_location(program, 0, "pos");
        gl.bind_attrib_location(program, 1, "uv");

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            error!("Error linkeando programa: {}", gl.get_program_info_log(program));
        }
        gl.delete_shader(vs);
        gl.delete_shader(fs);

        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 0.0,
             1.0, -1.0, 1.0, 0.0,
            -1.0,  1.0, 0.0, 1.0,
             1.0,  1.0, 1.0, 1.0,
        ];

        let vao_left = gl.create_vertex_array().unwrap();
        let vao_right = gl.create_vertex_array().unwrap();
        for vao in [vao_left, vao_right] {
            gl.bind_vertex_array(Some(vao));
            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck_cast(&vertices), glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);
        }

        (program, vao_left, vao_right)
    }

    pub fn dibujar_frame(&self) {
        // Preguntamos el tamaño real de la superficie en cada frame, en vez de
        // confiar en el ancho/alto que guardamos al crear el renderer -- ese
        // valor queda desactualizado si el modo inmersivo (o cualquier cambio
        // de insets/barras del sistema) redimensiona la ventana después.
        let ancho = self
            .egl
            .query_surface(self.display, self.surface, egl::WIDTH)
            .unwrap_or(self.ancho);
        let alto = self
            .egl
            .query_surface(self.display, self.surface, egl::HEIGHT)
            .unwrap_or(self.alto);

        unsafe {
            let gl = &self.gl;
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(self.program));
            let loc_color = gl.get_uniform_location(self.program, "color_ojo");

            gl.viewport(0, 0, ancho / 2, alto);
            gl.uniform_3_f32(loc_color.as_ref(), 1.0, 0.3, 0.3);
            gl.bind_vertex_array(Some(self.vao_left));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            gl.viewport(ancho / 2, 0, ancho / 2, alto);
            gl.uniform_3_f32(loc_color.as_ref(), 0.3, 0.3, 1.0);
            gl.bind_vertex_array(Some(self.vao_right));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        let _ = self.egl.swap_buffers(self.display, self.surface);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_surface(self.display, self.surface);
        let _ = self.egl.destroy_context(self.display, self.context);
    }
}

fn bytemuck_cast(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}