//! `vec2f!` —— WGSL 风格 `vec2<f32>` 构造宏(shader 与 CPU 通用)。
//!
//! - `vec2f!(s)`:单标量 → splat(展开成 `Vec2::splat`);
//! - `vec2f!(x, y)`:两个标量全填(展开成 `Vec2::new`)。
//!
//! 翻译器(gpu-macro)把它译成 WGSL `vec2<f32>(s, s)` / `vec2<f32>(x, y)`。

#[macro_export]
macro_rules! vec2f {
    ($s:expr$(,)?) => {
        $crate::Vec2::splat($s)
    };
    ($x:expr, $y:expr$(,)?) => {
        $crate::Vec2::new($x, $y)
    };
    ($($t:tt)*) => {
        compile_error!("vec2f! 只支持 vec2f!(s) 或 vec2f!(x, y)")
    };
}
