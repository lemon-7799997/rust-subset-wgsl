//! `mat3x3f!` —— WGSL 风格 `mat3x3<f32>` 构造宏(shader 与 CPU 通用)。
//!
//! - `mat3x3f!(s)`:单标量 → 对角矩阵(对角为 s,其余 0;展开成 `Mat3::from_diagonal`);
//! - `mat3x3f!(c0, c1, c2)`:三个列向量(展开成 `Mat3::from_cols`);
//! - `mat3x3f!(9 个标量,列主序)`:展开成 `Mat3::from_cols_array`。
//!
//! 翻译器(gpu-macro)译成 WGSL:对角矩阵会显式展开成对角文本,列/标量版原样透传。

#[macro_export]
macro_rules! mat3x3f {
    ($s:expr $(,)?) => {
        $crate::Mat3::from_diagonal($crate::Vec3::splat($s))
    };
    ($c0:expr, $c1:expr, $c2:expr $(,)?) => {
        $crate::Mat3::from_cols($c0, $c1, $c2)
    };
    ($a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr $(,)?) => {
        $crate::Mat3::from_cols_array(&[
            $a0, $a1, $a2, //
            $a3, $a4, $a5, //
            $a6, $a7, $a8, //
        ])
    };
    ($($t:tt)*) => {
        compile_error!(
            "mat3x3f! 只支持 mat3x3f!(s)(对角) / mat3x3f!(c0, c1, c2)(三个列向量) \
             / mat3x3f!(9 个标量,列主序)"
        )
    };
}
