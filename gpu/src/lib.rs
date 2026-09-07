//! # gpu —— WGSL 桩库(no-op, 只服务类型检查;向量/矩阵来自 glam)
//!
//! 目录结构(本 crate 只有 `src/lib.rs` 一个入口,逻辑按主题拆在子模块里):
//!
//! | 文件 | 内容 |
//! |---|---|
//! | `vec2.rs` / `vec3.rs` / `vec4.rs` | 构造宏 `vec2f!` / `vec3f!` / `vec4f!` |
//! | `mat2x2.rs` / `mat3x3.rs` / `mat4x4.rs` | 构造宏 `mat2x2f!` / `mat3x3f!` / `mat4x4f!` |
//! | `math.rs` | 数学自由函数桩(名字 = WGSL 名,泛型签名) |
//! | `texture.rs` | texture/sampler handle 家族、纹理函数、`TextureLoad` 重载模拟 |
//! | `arrays.rs` | `array<T>` 假容器 |
//! | `const_default.rs` | `ConstDefault` trait + 原生/glam impl |
//!
//! 向量/矩阵类型本体是 **glam 再导出**(真类型,CPU/GPU 同构):
//! `Vec2/Vec3/Vec4` → WGSL `vec2/vec3/vec4<f32>`、`Mat2/Mat3/Mat4` →
//! `mat2x2/mat3x3/mat4x4<f32>`、`IVec*/UVec*` → `vecN<i32>/vecN<u32>`,
//! 由翻译器(gpu-macro)的映射表译回 WGSL。
//!
//! **WGSL 风格构造宏**(CPU 与 `#[shader]` 函数体内通用,翻译器识别后输出
//! WGSL 构造器文本):
//! - `vec2f!(s)`/`vec3f!(s)`/`vec4f!(s)`:单参 = splat;
//! - `vec2f!(x, y)` 等:N 个标量全填;
//! - `mat2x2f!(s)` 等:单参 = 对角矩阵;
//! - `mat2x2f!(c0, c1)`/`mat3x3f!(c0, c1, c2)`/`mat4x4f!(c0..c3)`:按列传列向量;
//! - `mat4x4f!(16 个标量)` 等:`dim²` 个标量、列主序。
#![allow(non_camel_case_types, non_snake_case)]

// ---- 子模块(逻辑按主题拆细:vec/mat 各一个文件) ----
mod arrays;
mod const_default;
mod math;
mod mat2x2;
mod mat3x3;
mod mat4x4;
mod texture;
mod vec2;
mod vec3;
mod vec4;

// ---- 向量/矩阵类型 = glam(单一来源,真类型) ----
pub use glam::{IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};

// ---- 各子模块的公开项统一在根再导出(shader 里 `use gpu::*;` 一次引入) ----
pub use arrays::array;
pub use const_default::ConstDefault;
pub use math::*;
pub use texture::*;

// 注意:`vec2f!` 等构造宏用 `#[macro_export]` 定义在各自子模块里,
// 展开后自动挂到本 crate 根,`use gpu::*;`(或 `use gpu::vec4f;`)即可使用。
