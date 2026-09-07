//! `mat2x2f!` —— WGSL 风格 `mat2x2<f32>` 构造宏(shader 与 CPU 通用)。
//!
//! - `mat2x2f!(s)`:单标量 → 对角矩阵(对角为 s,其余 0;展开成 `Mat2::from_diagonal`);
//! - `mat2x2f!(c0, c1)`:两个列向量(展开成 `Mat2::from_cols`);
//! - `mat2x2f!(a0, a1, a2, a3)`:4 个标量、列主序(展开成 `Mat2::from_cols_array`)。
//!
//! 翻译器(gpu-macro)译成 WGSL:对角矩阵会显式展开成对角文本
//! `mat2x2<f32>(s, 0.0, 0.0, s)`,列/标量版原样透传。

#[macro_export]
macro_rules! mat2x2f {
    ($s:expr$(,)?) => {
        $crate::Mat2::from_diagonal($crate::Vec2::splat($s))
    };
    ($c0:expr, $c1:expr$(,)?) => {
        $crate::Mat2::from_cols($c0, $c1)
    };
    ($a0:expr, $a1:expr, $a2:expr, $a3:expr$(,)?) => {
        $crate::Mat2::from_cols_array(&[$a0, $a1, $a2, $a3])
    };
    ($($t:tt)*) => {
        compile_error!(
            "mat2x2f! 只支持 mat2x2f!(s)(对角) / mat2x2f!(c0, c1)(两个列向量) \
             / mat2x2f!(4 个标量,列主序)"
        )
    };
}
