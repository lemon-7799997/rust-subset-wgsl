//! `vec3f!` —— WGSL 风格 `vec3<f32>` 构造宏(shader 与 CPU 通用)。
//!
//! - `vec3f!(s)`:单标量 → splat(展开成 `Vec3::splat`);
//! - `vec3f!(x, y, z)`:三个标量全填(展开成 `Vec3::new`)。
//!
//! 翻译器(gpu-macro)把它译成 WGSL `vec3<f32>(s, s, s)` / `vec3<f32>(x, y, z)`。

#[macro_export]
macro_rules! vec3f {
    ($s:expr$(,)?) => {
        $crate::Vec3::splat($s)
    };
    ($x:expr, $y:expr, $z:expr$(,)?) => {
        $crate::Vec3::new($x, $y, $z)
    };
    ($($t:tt)*) => {
        compile_error!("vec3f! 只支持 vec3f!(s) 或 vec3f!(x, y, z)")
    };
}
