#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

extern crate glfw;

use self::glfw::{Context, Key, Action};

extern crate gl;

use self::gl::types::*;

use std::ptr;
use std::mem;
use std::os::raw::c_void;
use std::path::Path;
use std::ffi::CStr;

use image;
use image::GenericImage;
use image::DynamicImage::*;
use image::GenericImageView;

use common::{process_events, loadTexture};
use shader::Shader;
use camera::Camera;
use camera::Camera_Movement::*;

use cgmath::{Matrix4, vec3, Vector3, Deg, perspective, ortho, Point3};
use cgmath::prelude::*;

// settings
const SCR_WIDTH: u32 = 1280;
const SCR_HEIGHT: u32 = 720;
const SHADOW_WIDTH: i32 = 1024;
const SHADOW_HEIGHT: i32 = 1024;

pub fn main_5_3_1_2() {
    let mut camera = Camera {
        Position: Point3::new(0.0, 0.0, 3.0),
        ..Camera::default()
    };

    let mut firstMouse = true;
    let mut lastX: f32 = SCR_WIDTH as f32 / 2.0;
    let mut lastY: f32 = SCR_HEIGHT as f32 / 2.0;

    // timing
    let mut deltaTime: f32; // time between current frame and last frame
    let mut lastFrame: f32 = 0.0;

    // glfw: initialize and configure
    // ------------------------------
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    #[cfg(target_os = "macos")]
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    // glfw window creation
    // --------------------
    let (mut window, events) = glfw.create_window(SCR_WIDTH, SCR_HEIGHT, "LearnOpenGL", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window");

    window.make_current();
    window.set_framebuffer_size_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_scroll_polling(true);

    // tell GLFW to capture our mouse
    window.set_cursor_mode(glfw::CursorMode::Disabled);

    // gl: load all OpenGL function pointers
    // ---------------------------------------
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let (shader, simpleDepthShader, debugDepthQuad, planeVBO, planeVAO, woodTexture, depthMap, depthMapFBO) = unsafe {
        // configure global opengl state
        // -----------------------------
        gl::Enable(gl::DEPTH_TEST);

        // build and compile shaders
        // ------------------------------------
        let shader = Shader::new(
            "src/_5_advanced_lighting/shaders/3.1.2.shadow_mapping.vs",
            "src/_5_advanced_lighting/shaders/3.1.2.shadow_mapping.fs",
        );
        let simpleDepthShader = Shader::new(
            "src/_5_advanced_lighting/shaders/3.1.2.shadow_mapping_depth.vs",
            "src/_5_advanced_lighting/shaders/3.1.2.shadow_mapping_depth.fs");

        let debugDepthQuad = Shader::new(
            "src/_5_advanced_lighting/shaders/3.1.2.debug_quad.vs",
            "src/_5_advanced_lighting/shaders/3.1.2.debug_quad_depth.fs",
        );

        // set up vertex data (and buffer(s)) and configure vertex attributes
        // ------------------------------------------------------------------
        let planeVertices: [f32; 48] = [
            // positions            // normals         // texcoords
            25.0, -0.5, 25.0, 0.0, 1.0, 0.0, 25.0, 0.0,
            -25.0, -0.5, 25.0, 0.0, 1.0, 0.0, 0.0, 0.0,
            -25.0, -0.5, -25.0, 0.0, 1.0, 0.0, 0.0, 25.0,
            25.0, -0.5, 25.0, 0.0, 1.0, 0.0, 25.0, 0.0,
            -25.0, -0.5, -25.0, 0.0, 1.0, 0.0, 0.0, 25.0,
            25.0, -0.5, -25.0, 0.0, 1.0, 0.0, 25.0, 25.0,
        ];
        // plane VAO
        let (mut planeVAO, mut planeVBO) = (0, 0);
        gl::GenVertexArrays(1, &mut planeVAO);
        gl::GenBuffers(1, &mut planeVBO);
        gl::BindVertexArray(planeVAO);
        gl::BindBuffer(gl::ARRAY_BUFFER, planeVBO);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (planeVertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       &planeVertices[0] as *const f32 as *const c_void,
                       gl::STATIC_DRAW);
        gl::EnableVertexAttribArray(0);
        let stride = 8 * mem::size_of::<GLfloat>() as GLsizei;
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, stride, (3 * mem::size_of::<GLfloat>()) as *const c_void);
        gl::EnableVertexAttribArray(2);
        gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride, (6 * mem::size_of::<GLfloat>()) as *const c_void);
        gl::BindVertexArray(0);

        // load textures
        // -------------
        let woodTexture = loadTexture("resources/textures/wood.png");
        let mut depthMapFBO = 0;
        gl::GenFramebuffers(1, &mut depthMapFBO);
        let mut depthMap = 0;
        gl::GenTextures(1, &mut depthMap);
        gl::BindTexture(gl::TEXTURE_2D, depthMap);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT as i32, SHADOW_WIDTH, SHADOW_HEIGHT, 0, gl::DEPTH_COMPONENT, gl::FLOAT, ptr::null());
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);

        gl::BindFramebuffer(gl::FRAMEBUFFER, depthMapFBO);
        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, depthMap, 0);
        gl::DrawBuffer(gl::NONE);
        gl::ReadBuffer(gl::NONE);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        shader.useProgram();
        shader.setInt(c_str!("diffuseTexture"), 0);
        shader.setInt(c_str!("shadowMap"), 1);
        debugDepthQuad.useProgram();
        debugDepthQuad.setInt(c_str!("depthMap"), 0);


        (shader, simpleDepthShader, debugDepthQuad, planeVBO, planeVAO, woodTexture, depthMap, depthMapFBO)
    };

    let (cubeVAO, cubeVBO) = unsafe { initializeCube() };
    let (quadVAO, quadVBO) = unsafe { initializeQuad() };

    let lightPos = Point3::new(-2.0, 4.0, -1.0);

    // render loop
    // -----------
    while !window.should_close() {
        // per-frame time logic
        // --------------------
        let currentFrame = glfw.get_time() as f32;
        deltaTime = currentFrame - lastFrame;
        lastFrame = currentFrame;

        // events
        // -----
        process_events(&events, &mut firstMouse, &mut lastX, &mut lastY, &mut camera);

        // input
        // -----
        processInput(&mut window, deltaTime, &mut camera);

        // render
        // ------
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            let near_plane = 1.0f32;
            let far_plane = 7.5;
            let lightProjection = ortho(-10.0, 10.0, -10.0, 10.0, near_plane, far_plane);
            let lightView = Matrix4::<f32>::look_at(lightPos, Point3::new(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));
            let lightSpaceMatrix = lightProjection * lightView;
            simpleDepthShader.useProgram();
            simpleDepthShader.setMat4(c_str!("lightSpaceMatrix"), &lightSpaceMatrix);

            gl::Viewport(0, 0, SHADOW_WIDTH, SHADOW_HEIGHT);
            gl::BindFramebuffer(gl::FRAMEBUFFER, depthMapFBO);
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, woodTexture);
            renderScene(&simpleDepthShader, planeVAO, cubeVAO);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            gl::Viewport(0, 0, SCR_WIDTH as i32, SCR_HEIGHT as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            shader.useProgram();
            let projection = perspective(Deg(camera.Zoom), SCR_WIDTH as f32 / SCR_HEIGHT as f32, 0.1, 100.0);
            let view = camera.GetViewMatrix();
            shader.setMat4(c_str!("projection"), &projection);
            shader.setMat4(c_str!("view"), &view);
            shader.setVec3(c_str!("viewPos"), camera.Position.x, camera.Position.y, camera.Position.z);
            shader.setVec3(c_str!("lightPos"), lightPos.x, lightPos.y, lightPos.z);
            shader.setMat4(c_str!("lightSpaceMatrix"), &lightSpaceMatrix);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, woodTexture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, depthMap);
            renderScene(&shader, planeVAO, cubeVAO);

            debugDepthQuad.useProgram();
            debugDepthQuad.setFloat(c_str!("near_plane"), near_plane);
            debugDepthQuad.setFloat(c_str!("far_plane"), far_plane);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, depthMap);
            //renderQuad(quadVAO);
        }

        // glfw: swap buffers and poll IO events (keys pressed/released, mouse moved etc.)
        // -------------------------------------------------------------------------------
        window.swap_buffers();
        glfw.poll_events();
    }

    // optional: de-allocate all resources once they've outlived their purpose:
    // ------------------------------------------------------------------------
    unsafe {
        gl::DeleteVertexArrays(1, &planeVAO);
        gl::DeleteBuffers(1, &planeVBO);
        gl::DeleteVertexArrays(1, &cubeVAO);
        gl::DeleteBuffers(1, &cubeVBO);
        gl::DeleteVertexArrays(1, &quadVAO);
        gl::DeleteBuffers(1, &quadVBO);
    }
}

// NOTE: not the same version as in common.rs
pub fn processInput(window: &mut glfw::Window, deltaTime: f32, camera: &mut Camera) {
    if window.get_key(Key::Escape) == Action::Press {
        window.set_should_close(true)
    }

    if window.get_key(Key::W) == Action::Press {
        camera.ProcessKeyboard(FORWARD, deltaTime);
    }
    if window.get_key(Key::S) == Action::Press {
        camera.ProcessKeyboard(BACKWARD, deltaTime);
    }
    if window.get_key(Key::A) == Action::Press {
        camera.ProcessKeyboard(LEFT, deltaTime);
    }
    if window.get_key(Key::D) == Action::Press {
        camera.ProcessKeyboard(RIGHT, deltaTime);
    }
}

unsafe fn renderScene(shader: &Shader, planeVAO: u32, cubeVAO: u32) {
    let mut model = Matrix4::identity();
    shader.setMat4(c_str!("model"), &model);
    gl::BindVertexArray(planeVAO);
    gl::DrawArrays(gl::TRIANGLES, 0, 6);

    model = Matrix4::identity();
    model = model * Matrix4::from_translation(vec3(0.0, 1.5, 0.0));
    model = model * Matrix4::from_scale(0.5);
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);

    model = Matrix4::identity();
    model = model * Matrix4::from_translation(vec3(2.0, 0.0, 1.0));
    model = model * Matrix4::from_scale(0.5);
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);

    model = Matrix4::identity();
    model = model * Matrix4::from_translation(vec3(-1.0, 0.0, 2.0));
    model = model * Matrix4::from_axis_angle(vec3(1.0, 0.0, 1.0).normalize(), Deg(60.0));
    model = model * Matrix4::from_scale(0.25);
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);
}

unsafe fn renderCube(cubeVAO: u32) {
    gl::BindVertexArray(cubeVAO);
    gl::DrawArrays(gl::TRIANGLES, 0, 36);
    gl::BindVertexArray(0);
}

unsafe fn renderQuad(quadVAO: u32) {
    gl::BindVertexArray(quadVAO);
    gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
    gl::BindVertexArray(0);
}

unsafe fn initializeCube() -> (u32, u32) {
    let (mut cubeVAO, mut cubeVBO) = (0u32, 0u32);
    let vertices: [f32; 36 * 8] = [
        // back face
        -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, // bottom-left
        1.0, 1.0, -1.0, 0.0, 0.0, -1.0, 1.0, 1.0, // top-right
        1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 1.0, 0.0, // bottom-right
        1.0, 1.0, -1.0, 0.0, 0.0, -1.0, 1.0, 1.0, // top-right
        -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, // bottom-left
        -1.0, 1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 1.0, // top-left
        // front face
        -1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, // bottom-left
        1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, // bottom-right
        1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, // top-right
        1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, // top-right
        -1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, // top-left
        -1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, // bottom-left
        // left face
        -1.0, 1.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0, // top-right
        -1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 1.0, // top-left
        -1.0, -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 1.0, // bottom-left
        -1.0, -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 1.0, // bottom-left
        -1.0, -1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 0.0, // bottom-right
        -1.0, 1.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0, // top-right
        // right face
        1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, // top-left
        1.0, -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 1.0, // bottom-right
        1.0, 1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 1.0, // top-right
        1.0, -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 1.0, // bottom-right
        1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, // top-left
        1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, // bottom-left
        // bottom face
        -1.0, -1.0, -1.0, 0.0, -1.0, 0.0, 0.0, 1.0, // top-right
        1.0, -1.0, -1.0, 0.0, -1.0, 0.0, 1.0, 1.0, // top-left
        1.0, -1.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, // bottom-left
        1.0, -1.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, // bottom-left
        -1.0, -1.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, // bottom-right
        -1.0, -1.0, -1.0, 0.0, -1.0, 0.0, 0.0, 1.0, // top-right
        // top face
        -1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 0.0, 1.0, // top-left
        1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, // bottom-right
        1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 1.0, // top-right
        1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, // bottom-right
        -1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 0.0, 1.0, // top-left
        -1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0  // bottom-left
    ];
    gl::GenVertexArrays(1, &mut cubeVAO);
    gl::GenBuffers(1, &mut cubeVBO);

    gl::BindBuffer(gl::ARRAY_BUFFER, cubeVAO);
    gl::BufferData(gl::ARRAY_BUFFER,
                   (vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                   &vertices[0] as *const f32 as *const c_void,
                   gl::STATIC_DRAW);

    let stride = (8 * std::mem::size_of::<GLfloat>()) as GLsizei;

    gl::BindVertexArray(cubeVAO);
    gl::EnableVertexAttribArray(0);
    gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
    gl::EnableVertexAttribArray(1);
    gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, stride, (3 * std::mem::size_of::<GLfloat>()) as *const c_void);
    gl::EnableVertexAttribArray(2);
    gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride, (6 * std::mem::size_of::<GLfloat>()) as *const c_void);
    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    gl::BindVertexArray(0);

    (cubeVAO, cubeVBO)
}

unsafe fn initializeQuad() -> (u32, u32) {
    let (mut quadVAO, mut quadVBO) = (0u32, 0u32);

    let quadVertices: [f32; 20] = [
        // positions        // texture Coords
        -1.0, 1.0, 0.0, 0.0, 1.0,
        -1.0, -1.0, 0.0, 0.0, 0.0,
        1.0, 1.0, 0.0, 1.0, 1.0,
        1.0, -1.0, 0.0, 1.0, 0.0,
    ];

    gl::GenVertexArrays(1, &mut quadVAO);
    gl::GenBuffers(1, &mut quadVBO);
    gl::BindVertexArray(quadVAO);
    gl::BindBuffer(gl::ARRAY_BUFFER, quadVBO);
    gl::BufferData(gl::ARRAY_BUFFER,
                   (std::mem::size_of::<GLfloat>() * quadVertices.len()) as GLsizeiptr,
                   &quadVertices[0] as *const f32 as *const c_void,
                   gl::STATIC_DRAW,
    );

    let stride = (std::mem::size_of::<GLfloat>() * 5) as GLsizei;
    gl::EnableVertexAttribArray(0);
    gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, 0 as *const c_void);
    gl::EnableVertexAttribArray(1);
    gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, (3 * std::mem::size_of::<GLfloat>()) as *const c_void);

    (quadVAO, quadVBO)
}