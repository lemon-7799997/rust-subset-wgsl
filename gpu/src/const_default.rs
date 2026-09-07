// ============================================================================
// ConstDefault trait + 各类型 impl
// trait 在 gpu(本文件),derive 在 gpu-macro;使用处两者都要引入
// (derive 生成的是不带路径的 `impl ConstDefault`,trait 必须在作用域里)。
// ============================================================================

use core::marker::PhantomData;

use crate::{IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};

pub trait ConstDefault {
    const DEFAULT: Self;
}

impl<T> ConstDefault for PhantomData<T> {
    const DEFAULT: Self = PhantomData;
}

// 为所有数值类型批量实现
macro_rules! impl_const_default_for_numeric {
    ($($ty:ty),*) => {
        $(
            impl ConstDefault for $ty {
                const DEFAULT: Self = 0;
            }
        )*
    };
}

impl_const_default_for_numeric! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize
}

impl ConstDefault for f32 {
    const DEFAULT: Self = 0.0;
}

impl ConstDefault for f64 {
    const DEFAULT: Self = 0.0;
}

impl ConstDefault for bool {
    const DEFAULT: Self = false;
}

impl ConstDefault for char {
    const DEFAULT: Self = '\0';
}

impl ConstDefault for String {
    const DEFAULT: Self = String::new();
}

impl ConstDefault for &'static str {
    const DEFAULT: Self = "";
}

// ---- glam 类型也要能当"默认初值"(static 初值、array<T> 元素) ----
// gpu 定义了 ConstDefault(本地 trait),所以在这里给外来 glam 类型补 impl
// 不违反孤儿规则。初值只用于 Rust 侧 static 初始化,翻译时被丢弃。

macro_rules! impl_const_default_glam {
    ($($ty:ty => $default:expr),+ $(,)?) => {
        $(
            impl ConstDefault for $ty {
                const DEFAULT: Self = $default;
            }
        )+
    };
}

impl_const_default_glam! {
    Vec2 => Vec2::ZERO,
    Vec3 => Vec3::ZERO,
    Vec4 => Vec4::ZERO,
    IVec2 => IVec2::ZERO,
    IVec3 => IVec3::ZERO,
    IVec4 => IVec4::ZERO,
    UVec2 => UVec2::ZERO,
    UVec3 => UVec3::ZERO,
    UVec4 => UVec4::ZERO,
    Mat2 => Mat2::IDENTITY,
    Mat3 => Mat3::IDENTITY,
    Mat4 => Mat4::IDENTITY,
}
