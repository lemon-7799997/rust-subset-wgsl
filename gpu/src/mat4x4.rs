//! `mat4x4f!` —— WGSL 风格 `mat4x4<f32>` 构造宏(shader 与 CPU 通用)。
//!
//! - `mat4x4f!(s)`:单标量 → 对角矩阵(对角为 s,其余 0;展开成 `Mat4::from_diagonal`);
//! - `mat4x4f!(c0, c1, c2, c3)`:四个列向量(展开成 `Mat4::from_cols`);
//! - `mat4x4f!(16 个标量,列主序)`:展开成 `Mat4::from_cols_array`。
//!
//! 翻译器(gpu-macro)译成 WGSL:对角矩阵会显式展开成对角文本,列/标量版原样透传。

#[macro_export]
macro_rules! mat4x4f {
    ($s:expr $(,)?) => {
        $crate::Mat4::from_diagonal($crate::Vec4::splat($s))
    };
    ($c0:expr, $c1:expr, $c2:expr, $c3:expr $(,)?) => {
        $crate::Mat4::from_cols($c0, $c1, $c2, $c3)
    };
    ($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr, $a9:expr, $a10:expr, $a11:expr, $a12:expr, $a13:expr, $a14:expr, $a15:expr $(,)?) => {
        $crate::Mat4::from_cols_array(&[
            $a0, $a1, $a2, $a3, //
            $a4, $a5, $a6, $a7, //
            $a8, $a9, $a10, $a11, //
            $a12, $a13, $a14, $a15, //
        ])
    };
    ($($t:tt)*) => {
        compile_error!(
            "mat4x4f! 只支持 mat4x4f!(s)(对角) / mat4x4f!(c0..c3)(四个列向量) \
             / mat4x4f!(16 个标量,列主序)"
        )
    };
}
