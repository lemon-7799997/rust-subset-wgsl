//! `vec4f!` —— WGSL 风格 `vec4<f32>` 构造宏(shader 与 CPU 通用)。
//!
//! - `vec4f!(s)`:单标量 → splat(展开成 `Vec4::splat`);
//! - `vec4f!(x, y, z, w)`:四个标量全填(展开成 `Vec4::new`)。
//!
//! 翻译器(gpu-macro)把它译成 WGSL
//! `vec4<f32>(s, s, s, s)` / `vec4<f32>(x, y, z, w)`。

#[macro_export]
macro_rules! vec4f {
    ($s:expr$(,)?) => {
        $crate::Vec4::splat($s)
    };
    ($x:expr, $y:expr, $z:expr, $w:expr$(,)?) => {
        $crate::Vec4::new($x, $y, $z, $w)
    };
    ($($t:tt)*) => {
        compile_error!("vec4f! 只支持 vec4f!(s) 或 vec4f!(x, y, z, w)")
    };
}
