//! # gpu —— WGSL 桩库(no-op, 只服务类型检查)
//!
//! 这里的每个名字都对应一个 WGSL 里的名字。翻译器(`gpu-macro`)看到它们
//! 时原样照抄,所以:
//!   - 类型名/函数名**必须**和 WGSL 完全一致(因此才全是小写,靠
//!     `non_camel_case_types` 豁免 lint);
//!   - 函数体永远不该被执行,全部 `unimplemented!()`;
//!   - 签名目前故意很松(泛型不加约束),以后再用 trait 收紧,
//!     让"错得离谱"的调用在 Rust 侧就报错。
#![allow(non_camel_case_types, non_snake_case)]

use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Sub};

// ============================================================================
// 向量类型: vec2<T> / vec3<T> / vec4<T>
// 字段名 = WGSL 字段访问名(x/y/z/w),翻译时原样保留。
// ============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct vec2<T> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

// 构造函数: 类型在 type namespace, 函数在 value namespace, 可以同名。
// 调用点写成 vec2::<f32>(x, y)(turbofish),翻译时把 `::<` 换成 `<` 就是
// WGSL 的 vec2<f32>(x, y);元素类型显式写在代码里,翻译器不用猜。
#[inline]
pub const fn vec2<T>(x: T, y: T) -> vec2<T> {
    vec2 { x, y }
}

#[inline]
pub const fn vec3<T>(x: T, y: T, z: T) -> vec3<T> {
    vec3 { x, y, z }
}

#[inline]
pub const fn vec4<T>(x: T, y: T, z: T, w: T) -> vec4<T> {
    vec4 { x, y, z, w }
}

// 运算符: 分量级 + - * /,以及 *f32 /f32(标量只支持 f32,后面需要再加)
macro_rules! impl_vec_ops {
    ($v:ident { $($f:ident),+ }) => {
        impl<T: Copy + Add<Output = T>> Add for $v<T> {
            type Output = Self;
            #[inline]
            fn add(self, o: Self) -> Self {
                $v { $($f: self.$f + o.$f),+ }
            }
        }
        impl<T: Copy + Sub<Output = T>> Sub for $v<T> {
            type Output = Self;
            #[inline]
            fn sub(self, o: Self) -> Self {
                $v { $($f: self.$f - o.$f),+ }
            }
        }
        impl<T: Copy + Mul<Output = T>> Mul for $v<T> {
            type Output = Self;
            #[inline]
            fn mul(self, o: Self) -> Self {
                $v { $($f: self.$f * o.$f),+ }
            }
        }
        impl<T: Copy + Div<Output = T>> Div for $v<T> {
            type Output = Self;
            #[inline]
            fn div(self, o: Self) -> Self {
                $v { $($f: self.$f / o.$f),+ }
            }
        }
        impl<T: Copy + Mul<f32, Output = T>> Mul<f32> for $v<T> {
            type Output = Self;
            #[inline]
            fn mul(self, s: f32) -> Self {
                $v { $($f: self.$f * s),+ }
            }
        }
        impl<T: Copy + Div<f32, Output = T>> Div<f32> for $v<T> {
            type Output = Self;
            #[inline]
            fn div(self, s: f32) -> Self {
                $v { $($f: self.$f / s),+ }
            }
        }
    };
}

impl_vec_ops!(vec2 { x, y });
impl_vec_ops!(vec3 { x, y, z });
impl_vec_ops!(vec4 { x, y, z, w });

// ============================================================================
// 常用数学函数(no-op 桩)
// TODO: 签名太松(比如 clamp 应该区分 vec/scalar 混用),以后用 trait 收紧。
// ============================================================================

/// 一元标量/向量函数,一一对应 WGSL 内建。
macro_rules! unary_fn {
    ($($name:ident),+ $(,)?) => {
        $(pub fn $name<T>(v: T) -> T {
            let _ = v;
            unimplemented!("no-op stub: only for type checking")
        })+
    };
}

unary_fn!(
    abs, sign, fract, floor, ceil, round, trunc, sqrt, inverse_sqrt, exp, exp2,
    log, log2, sin, cos, tan, asin, acos, atan, radians, degrees,
);

/// 二元函数(两个同类型参数)。
macro_rules! binary_fn {
    ($($name:ident),+ $(,)?) => {
        $(pub fn $name<T>(a: T, b: T) -> T {
            let _ = (a, b);
            unimplemented!("no-op stub: only for type checking")
        })+
    };
}

binary_fn!(min, max, step, pow, atan2);

/// 三元函数。
pub fn clamp<T>(v: T, lo: T, hi: T) -> T {
    let _ = (v, lo, hi);
    unimplemented!("no-op stub: only for type checking")
}

pub fn mix<T>(a: T, b: T, t: f32) -> T {
    let _ = (a, b, t);
    unimplemented!("no-op stub: only for type checking")
}

pub fn smoothstep<T>(lo: T, hi: T, v: T) -> T {
    let _ = (lo, hi, v);
    unimplemented!("no-op stub: only for type checking")
}

// ---- 向量专用(签名以后收紧成"只接受向量") ----

pub fn dot<T>(a: T, b: T) -> f32 {
    let _ = (a, b);
    unimplemented!("no-op stub: only for type checking")
}

pub fn cross<T>(a: T, b: T) -> T {
    let _ = (a, b);
    unimplemented!("no-op stub: only for type checking")
}

pub fn length<T>(v: T) -> f32 {
    let _ = v;
    unimplemented!("no-op stub: only for type checking")
}

pub fn distance<T>(a: T, b: T) -> f32 {
    let _ = (a, b);
    unimplemented!("no-op stub: only for type checking")
}

pub fn normalize<T>(v: T) -> T {
    let _ = v;
    unimplemented!("no-op stub: only for type checking")
}

pub fn faceforward<T>(n: T, i: T, nref: T) -> T {
    let _ = (n, i, nref);
    unimplemented!("no-op stub: only for type checking")
}

pub fn reflect<T>(i: T, n: T) -> T {
    let _ = (i, n);
    unimplemented!("no-op stub: only for type checking")
}

pub fn refract<T>(i: T, n: T, eta: f32) -> T {
    let _ = (i, n, eta);
    unimplemented!("no-op stub: only for type checking")
}

// ============================================================================
// Handle 类型(纹理/采样器)
// WGSL 里它们声明在模块级,但没有地址空间(不是 var<uniform>/<storage>):
//   @group(0) @binding(1) var tex: texture_2d<f32>;
//   @group(0) @binding(2) var smp: sampler;
// 翻译器看到 texture_2d / sampler 类型时会跳过地址空间部分。
// ============================================================================

/// 2D 纹理(handle)。`new()` 只用于 Rust 侧 static 初始化,翻译时初值被丢弃。
#[derive(Clone, Copy)]
pub struct texture_2d<T>(PhantomData<T>);

impl<T> texture_2d<T> {
    #[inline]
    pub const fn new() -> Self {
        texture_2d(PhantomData)
    }
}

/// 采样器(handle)。unit struct,值就是它自己。
#[derive(Clone, Copy)]
pub struct sampler;

// ---- 常用纹理函数(no-op 桩,1:1 透传) ----

pub fn textureSample<T>(t: texture_2d<T>, s: sampler, uv: vec2<f32>) -> vec4<f32> {
    let _ = (t, s, uv);
    unimplemented!("no-op stub: only for type checking")
}

pub fn textureLoad<T>(t: texture_2d<T>, coords: vec2<i32>, level: i32) -> vec4<f32> {
    let _ = (t, coords, level);
    unimplemented!("no-op stub: only for type checking")
}

pub fn textureDimensions<T>(t: texture_2d<T>, level: i32) -> vec2<u32> {
    let _ = (t, level);
    unimplemented!("no-op stub: only for type checking")
}

// ============================================================================
// array<T, N>:真容器(桩库规则:只实现"必须的功能")
// 存真实元素、支持下标读写(usize + u32)——没有这些重放副本就没法通过
// rustc 类型检查。WGSL 里数组下标是 u32/i32,所以直接支持 u32 下标,
// 让 Rust 侧写法和 WGSL 源一致。数学类内建仍保持 no-op。
// ============================================================================

pub struct array<T, const N: usize> {
    elems: [T; N],
}

impl<T, const N: usize> array<T, N> {
    /// 从 Rust 定长数组构造(给 static / 局部提供可类型检查的值;
    /// 翻译后数组是 WGSL 声明,初值会被丢弃)。
    #[inline]
    pub const fn from_arr(elems: [T; N]) -> Self {
        array { elems }
    }
}

impl<T, const N: usize> core::ops::Index<usize> for array<T, N> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        &self.elems[i]
    }
}

impl<T, const N: usize> core::ops::IndexMut<usize> for array<T, N> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.elems[i]
    }
}

// WGSL 风格:直接用 u32 下标(不需要 `as usize`)
impl<T, const N: usize> core::ops::Index<u32> for array<T, N> {
    type Output = T;
    #[inline]
    fn index(&self, i: u32) -> &T {
        &self.elems[i as usize]
    }
}

impl<T, const N: usize> core::ops::IndexMut<u32> for array<T, N> {
    #[inline]
    fn index_mut(&mut self, i: u32) -> &mut T {
        &mut self.elems[i as usize]
    }
}
