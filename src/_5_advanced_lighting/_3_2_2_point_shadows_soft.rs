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
use std::ffi::{CStr, CString};
use std::ops::Add;

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
use self::glfw::ffi::glfwGetTime;

// settings
const SCR_WIDTH: u32 = 1280;
const SCR_HEIGHT: u32 = 720;
const SHADOW_WIDTH: i32 = 1024;
const SHADOW_HEIGHT: i32 = 1024;

pub fn main_5_3_2_2() {
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

    let (shader, simpleDepthShader, woodTexture, depthCubemap, depthMapFBO) = unsafe {
        // configure global opengl state
        // -----------------------------
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);

        // build and compile shaders
        // ------------------------------------
        let shader = Shader::new(
            "src/_5_advanced_lighting/shaders/3.2.2.point_shadows.vs",
            "src/_5_advanced_lighting/shaders/3.2.2.point_shadows.fs",
        );
        let simpleDepthShader = Shader::with_geometry_shader(
            "src/_5_advanced_lighting/shaders/3.2.2.point_shadows_depth.vs",
            "src/_5_advanced_lighting/shaders/3.2.2.point_shadows_depth.fs",
            "src/_5_advanced_lighting/shaders/3.2.2.point_shadows_depth.gs",
        );

        // load textures
        // -------------
        let woodTexture = loadTexture("resources/textures/wood.png");
        let mut depthMapFBO = 0;
        gl::GenFramebuffers(1, &mut depthMapFBO);
        let mut depthCubemap = 0;
        gl::GenTextures(1, &mut depthCubemap);
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, depthCubemap);
        for i in 0..6 {
            gl::TexImage2D(gl::TEXTURE_CUBE_MAP_POSITIVE_X + i, 0, gl::DEPTH_COMPONENT as i32, SHADOW_WIDTH, SHADOW_HEIGHT, 0, gl::DEPTH_COMPONENT, gl::FLOAT, ptr::null());
        }
        gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as i32);

        gl::BindFramebuffer(gl::FRAMEBUFFER, depthMapFBO);
        gl::FramebufferTexture(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, depthCubemap, 0);
        gl::DrawBuffer(gl::NONE);
        gl::ReadBuffer(gl::NONE);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        shader.useProgram();
        shader.setInt(c_str!("diffuseTexture"), 0);
        shader.setInt(c_str!("depthMap"), 1);

        (shader, simpleDepthShader, woodTexture, depthCubemap, depthMapFBO)
    };

    let (cubeVAO, cubeVBO) = unsafe { initializeCube() };

    let mut lightPos = Point3::new(0.0f32, 0.0, 0.0);
    let mut shadowsKeyPressed = false;
    let mut shadows = false;

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
        processInput(&mut window, deltaTime, &mut camera, &mut shadowsKeyPressed, &mut shadows);


        // render
        // ------
        unsafe {
            lightPos.z = ((glfwGetTime() * 0.5).sin() * 3.0) as f32;

            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            let near_plane = 1.0f32;
            let far_plane = 25.0;
            let shadowProj = perspective(Deg(90.0), SHADOW_WIDTH as f32 / SHADOW_HEIGHT as f32, near_plane, far_plane);
            let mut shadowTransforms: Vec<Matrix4<f32>> = Vec::new();
            shadowTransforms.push(shadowProj * Matrix4::look_at(lightPos, lightPos + vec3(1.0, 0.0, 0.0), vec3(0.0, -1.0, 0.0)));
            shadowTransforms.push(shadowProj * Matrix4::look_at(lightPos, lightPos + vec3(-1.0, 0.0, 0.0), vec3(0.0, -1.0, 0.0)));
            shadowTransforms.push(shadowProj * Matrix4::look_at(lightPos, lightPos + vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0)));
            shadowTransforms.push(shadowProj * Matrix4::look_at(lightPos, lightPos + vec3(0.0, -1.0, 0.0), vec3(0.0, 0.0, -1.0)));
            shadowTransforms.push(shadowProj * Matrix4::look_at(lightPos, lightPos + vec3(0.0, 0.0, 1.0), vec3(0.0, -1.0, 0.0)));
            shadowTransforms.push(shadowProj * Matrix4::look_at(lightPos, lightPos + vec3(0.0, 0.0, -1.0), vec3(0.0, -1.0, 0.0)));

            gl::Viewport(0, 0, SHADOW_WIDTH, SHADOW_HEIGHT);
            gl::BindFramebuffer(gl::FRAMEBUFFER, depthMapFBO);
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            simpleDepthShader.useProgram();
            for i in 0..6 {
                let key = CString::new(format!("shadowMatrices[{}]", i)).unwrap();
                simpleDepthShader.setMat4(&key, &shadowTransforms[i]);
            }
            simpleDepthShader.setFloat(c_str!("far_plane"), far_plane);
            simpleDepthShader.setVec3(c_str!("lightPos"), lightPos.x, lightPos.y, lightPos.z);
            renderScene(&simpleDepthShader, cubeVAO);
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
            shader.setInt(c_str!("shadows"), shadows as i32);
            shader.setFloat(c_str!("far_plane"), far_plane);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, woodTexture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_CUBE_MAP, depthCubemap);
            renderScene(&shader, cubeVAO);
        }

        // glfw: swap buffers and poll IO events (keys pressed/released, mouse moved etc.)
        // -------------------------------------------------------------------------------
        window.swap_buffers();
        glfw.poll_events();
    }

    // optional: de-allocate all resources once they've outlived their purpose:
    // ------------------------------------------------------------------------
    unsafe {
        gl::DeleteVertexArrays(1, &cubeVAO);
        gl::DeleteBuffers(1, &cubeVBO);
    }
}

// NOTE: not the same version as in common.rs
pub fn processInput(window: &mut glfw::Window, deltaTime: f32, camera: &mut Camera, shadowsKeyPressed: &mut bool, shadows: &mut bool) {
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

    if window.get_key(Key::Space) == Action::Press && !*shadowsKeyPressed {
        *shadows = !*shadows;
        *shadowsKeyPressed = true;
    }

    if window.get_key(Key::Space) == Action::Release {
        *shadowsKeyPressed = false;
    }
}

unsafe fn renderScene(shader: &Shader, cubeVAO: u32) {
    let mut model = Matrix4::identity();
    model = Matrix4::from_scale(5.0) * model;
    shader.setMat4(c_str!("model"), &model);
    gl::Disable(gl::CULL_FACE);
    shader.setInt(c_str!("reverse_normals"), 1);
    renderCube(cubeVAO);

    shader.setInt(c_str!("reverse_normals"), 0);
    gl::Enable(gl::CULL_FACE);

    model = Matrix4::identity();
    model = Matrix4::from_translation(vec3(4.0, -3.5, 0.0)) * model;
    model = Matrix4::from_scale(0.5) * model;
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);

    model = Matrix4::identity();
    model = Matrix4::from_translation(vec3(2.0, 3.0, 1.0)) * model;
    model = Matrix4::from_scale(0.75) * model;
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);

    model = Matrix4::identity();
    model = Matrix4::from_translation(vec3(-3.0, -1.0, 0.0)) * model;
    model = Matrix4::from_scale(0.75) * model;
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);

    model = Matrix4::identity();
    model = Matrix4::from_translation(vec3(-1.5, 1.0, 1.5)) * model;
    model = Matrix4::from_scale(0.75) * model;
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);

    model = Matrix4::identity();
    model = Matrix4::from_translation(vec3(-1.5, 2.0, -3.0)) * model;
    model = Matrix4::from_axis_angle(vec3(1.0, 0.0, 1.0).normalize(), Deg(60.0)) * model;
    model = Matrix4::from_scale(0.75) * model;
    shader.setMat4(c_str!("model"), &model);
    renderCube(cubeVAO);
}

unsafe fn renderCube(cubeVAO: u32) {
    gl::BindVertexArray(cubeVAO);
    gl::DrawArrays(gl::TRIANGLES, 0, 36);
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

