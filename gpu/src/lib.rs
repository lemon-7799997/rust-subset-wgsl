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
use core::str;

// ============================================================================
// 向量类型: vec2<T> / vec3<T> / vec4<T>
// 字段名 = WGSL 字段访问名(x/y/z/w),翻译时原样保留。
// ============================================================================

#[derive(Clone, Copy, Debug, Default, gpu_macro::ConstDefault, PartialEq)]
pub struct vec2<T> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Copy, Debug, Default, gpu_macro::ConstDefault, PartialEq)]
pub struct vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[derive(Clone, Copy, Debug, Default, gpu_macro::ConstDefault, PartialEq)]
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
    abs,
    sign,
    fract,
    floor,
    ceil,
    round,
    trunc,
    sqrt,
    inverse_sqrt,
    exp,
    exp2,
    log,
    log2,
    sin,
    cos,
    tan,
    asin,
    acos,
    atan,
    radians,
    degrees,
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
// 翻译器看到这些 handle 类型时会跳过地址空间部分。
// ============================================================================

/// 2D 纹理(handle)。`new()` 只用于 Rust 侧 static 初始化,翻译时初值被丢弃。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct texture_2d<T>(PhantomData<T>);

impl<T> texture_2d<T> {
    #[inline]
    pub const fn new() -> Self {
        texture_2d(PhantomData)
    }
}

/// 2D 纹理数组(handle,按 array_index 采样)。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct texture_2d_array<T>(PhantomData<T>);

impl<T> texture_2d_array<T> {
    #[inline]
    pub const fn new() -> Self {
        texture_2d_array(PhantomData)
    }
}

/// 立方体纹理(handle,按 vec3 方向采样)。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct texture_cube<T>(PhantomData<T>);

impl<T> texture_cube<T> {
    #[inline]
    pub const fn new() -> Self {
        texture_cube(PhantomData)
    }
}

/// 1D 纹理(handle)。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct texture_1d<T>(PhantomData<T>);

impl<T> texture_1d<T> {
    #[inline]
    pub const fn new() -> Self {
        texture_1d(PhantomData)
    }
}

/// 深度纹理(handle,无格式参数)。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct texture_depth_2d(PhantomData<f32>);

impl texture_depth_2d {
    #[inline]
    pub const fn new() -> Self {
        texture_depth_2d(PhantomData)
    }
}

/// 采样器(handle)。unit struct,值就是它自己。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct sampler;

/// 比较采样器(handle,配 depth 纹理做 shadow 采样)。
#[derive(Clone, Copy, gpu_macro::ConstDefault)]
pub struct sampler_comparison;

// ---- 常用纹理函数(no-op 桩) ----
//
// 注意: WGSL 的 textureSample/textureLoad/textureDimensions 是按参数类型
// 重载的;Rust 没有重载 → 同 WGSL 名的不同签名必须拆成不同 Rust 名
// (texture_sample_cube / texture_sample_array / ...),翻译器按改名表把它们
// 映射回 WGSL 名(见 gpu-macro 里 map_wgsl_name)。

pub fn textureSample<T>(t: texture_2d<T>, s: sampler, uv: vec2<f32>) -> vec4<f32> {
    let _ = (t, s, uv);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_cube<T>(t: texture_cube<T>, s: sampler, uvw: vec3<f32>) -> vec4<f32> {
    let _ = (t, s, uvw);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_array<T>(
    t: texture_2d_array<T>,
    s: sampler,
    uv: vec2<f32>,
    layer: i32,
) -> vec4<f32> {
    let _ = (t, s, uv, layer);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_depth(t: texture_depth_2d, s: sampler, uv: vec2<f32>) -> f32 {
    let _ = (t, s, uv);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_compare(
    t: texture_depth_2d,
    s: sampler_comparison,
    uv: vec2<f32>,
    depth_ref: f32,
) -> f32 {
    let _ = (t, s, uv, depth_ref);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_load_cube<T>(t: texture_cube<T>, coords: vec3<i32>, level: i32) -> vec4<f32> {
    let _ = (t, coords, level);
    unimplemented!("no-op stub: only for type checking")
}

pub fn textureDimensions<T>(t: texture_2d<T>, level: i32) -> vec2<u32> {
    let _ = (t, level);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_dimensions_array<T>(t: texture_2d_array<T>, level: i32) -> vec2<u32> {
    let _ = (t, level);
    unimplemented!("no-op stub: only for type checking")
}

// ============================================================================
// array<T>:假容器(桩库规则:只实现"必须的功能")
// 只放一个 phantom 元素,Index/IndexMut 忽略下标——目的纯粹是让重放副本
// 能过 rustc 类型检查;真正的数组是 WGSL 侧的 storage buffer 成员。
// WGSL 数组下标是 u32/i32,所以支持 usize/u32 下标,让源写法贴近 WGSL。
// ============================================================================

pub trait ConstDefault {
    const DEFAULT: Self;
}

impl<T> ConstDefault for PhantomData<T> {
    const DEFAULT: Self = PhantomData;
}

#[derive(gpu_macro::ConstDefault)]
pub struct array<T: ConstDefault> {
    phantom: T,
}

impl<T: ConstDefault> array<T> {}

impl<T: ConstDefault> core::ops::Index<usize> for array<T> {
    type Output = T;
    #[inline]
    fn index(&self, _i: usize) -> &T {
        &self.phantom
    }
}

impl<T: ConstDefault> core::ops::IndexMut<usize> for array<T> {
    #[inline]
    fn index_mut(&mut self, _i: usize) -> &mut T {
        &mut self.phantom
    }
}

// WGSL 风格:直接用 u32 下标(不需要 `as usize`)
impl<T: ConstDefault> core::ops::Index<u32> for array<T> {
    type Output = T;
    #[inline]
    fn index(&self, _i: u32) -> &T {
        &self.phantom
    }
}

impl<T: ConstDefault> core::ops::IndexMut<u32> for array<T> {
    #[inline]
    fn index_mut(&mut self, _i: u32) -> &mut T {
        &mut self.phantom
    }
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

// 调用宏
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

// ============================================================================
// mat4x4<T>:4x4 矩阵(真容器,列主序,按"必须的功能"口径实现 矩阵×向量)
// 字段 c0..c3 = 4 个列向量;构造器 16 个标量(列主序),翻译后就是 WGSL 的
// mat4x4<f32>(...),参数顺序文本直通。
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, gpu_macro::ConstDefault)]
pub struct mat4x4<T> {
    pub c0: vec4<T>,
    pub c1: vec4<T>,
    pub c2: vec4<T>,
    pub c3: vec4<T>,
}

/// WGSL mat4x4<f32>(...) 构造器;Rust 侧写 mat4x4::<f32>(16 个标量)。
#[inline]
pub const fn mat4x4<T>(
    a0: T, a1: T, a2: T, a3: T, //
    a4: T, a5: T, a6: T, a7: T, //
    a8: T, a9: T, a10: T, a11: T, //
    a12: T, a13: T, a14: T, a15: T, //
) -> mat4x4<T> {
    mat4x4 {
        c0: vec4::<T>(a0, a1, a2, a3),
        c1: vec4::<T>(a4, a5, a6, a7),
        c2: vec4::<T>(a8, a9, a10, a11),
        c3: vec4::<T>(a12, a13, a14, a15),
    }
}

// 矩阵 × 向量(列主序:M*v = Σ c_i * v_i)
impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul<vec4<T>> for mat4x4<T> {
    type Output = vec4<T>;
    #[inline]
    fn mul(self, v: vec4<T>) -> vec4<T> {
        vec4 {
            x: self.c0.x * v.x + self.c1.x * v.y + self.c2.x * v.z + self.c3.x * v.w,
            y: self.c0.y * v.x + self.c1.y * v.y + self.c2.y * v.z + self.c3.y * v.w,
            z: self.c0.z * v.x + self.c1.z * v.y + self.c2.z * v.z + self.c3.z * v.w,
            w: self.c0.w * v.x + self.c1.w * v.y + self.c2.w * v.z + self.c3.w * v.w,
        }
    }
}

// ============================================================================
// mat2x2<T> / mat3x3<T>(补齐矩阵家族,mat4x4 在上方)
// 列主序真容器:构造器标量个数 = N*N;支持 矩阵×向量 与 矩阵×矩阵。
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, gpu_macro::ConstDefault)]
pub struct mat2x2<T> {
    pub c0: vec2<T>,
    pub c1: vec2<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, gpu_macro::ConstDefault)]
pub struct mat3x3<T> {
    pub c0: vec3<T>,
    pub c1: vec3<T>,
    pub c2: vec3<T>,
}

/// WGSL mat2x2<f32>(...) 构造器(4 个标量,列主序)。
#[inline]
pub const fn mat2x2<T>(a0: T, a1: T, a2: T, a3: T) -> mat2x2<T> {
    mat2x2 {
        c0: vec2::<T>(a0, a1),
        c1: vec2::<T>(a2, a3),
    }
}

/// WGSL mat3x3<f32>(...) 构造器(9 个标量,列主序)。
#[inline]
pub const fn mat3x3<T>(
    a0: T, a1: T, a2: T, //
    a3: T, a4: T, a5: T, //
    a6: T, a7: T, a8: T, //
) -> mat3x3<T> {
    mat3x3 {
        c0: vec3::<T>(a0, a1, a2),
        c1: vec3::<T>(a3, a4, a5),
        c2: vec3::<T>(a6, a7, a8),
    }
}

// mat2x2 × vec2
impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul<vec2<T>> for mat2x2<T> {
    type Output = vec2<T>;
    #[inline]
    fn mul(self, v: vec2<T>) -> vec2<T> {
        vec2 {
            x: self.c0.x * v.x + self.c1.x * v.y,
            y: self.c0.y * v.x + self.c1.y * v.y,
        }
    }
}

// mat3x3 × vec3
impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul<vec3<T>> for mat3x3<T> {
    type Output = vec3<T>;
    #[inline]
    fn mul(self, v: vec3<T>) -> vec3<T> {
        vec3 {
            x: self.c0.x * v.x + self.c1.x * v.y + self.c2.x * v.z,
            y: self.c0.y * v.x + self.c1.y * v.y + self.c2.y * v.z,
            z: self.c0.z * v.x + self.c1.z * v.y + self.c2.z * v.z,
        }
    }
}

// 矩阵 × 矩阵(列主序:(A*B) 的第 j 列 = A * B 的第 j 列)
impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul for mat2x2<T> {
    type Output = Self;
    #[inline]
    fn mul(self, o: Self) -> Self {
        mat2x2 {
            c0: self * o.c0,
            c1: self * o.c1,
        }
    }
}

impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul for mat3x3<T> {
    type Output = Self;
    #[inline]
    fn mul(self, o: Self) -> Self {
        mat3x3 {
            c0: self * o.c0,
            c1: self * o.c1,
            c2: self * o.c2,
        }
    }
}

impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul for mat4x4<T> {
    type Output = Self;
    #[inline]
    fn mul(self, o: Self) -> Self {
        mat4x4 {
            c0: self * o.c0,
            c1: self * o.c1,
            c2: self * o.c2,
            c3: self * o.c3,
        }
    }
}

// ============================================================================
// 实验 v2:用「自由函数接收元组 + trait 分发」模拟函数重载
// ============================================================================
// WGSL 的 textureLoad 按纹理类型有多个签名,而 Rust 没有函数重载。
// 这里让 textureLoad 整体接收一个元组,trait 按元组形状选实现:
//
//   textureLoad((texture_1d<ST>,       C,        L))
//   textureLoad((texture_2d<ST>,       vec2<C>,  L))
//   textureLoad((texture_2d_array<ST>, vec2<C>,  A, L))
//
// 调用点写法与 WGSL 名完全一致(只是参数变成"一整包元组");
// rustc 靠 A: TextureLoad 约束检查元组形状/类型;
// 翻译器(gpu-macro)看到 textureLoad((…)) 会把元组摊平成 textureLoad(…)。
// ============================================================================

pub trait TextureLoad {
    /// 每个签名自己的返回类型(纹理元素类型 ST 决定 vec4<ST>)。
    type Output;
    fn load(self) -> Self::Output;
}

impl<ST, C, L> TextureLoad for (texture_1d<ST>, C, L) {
    type Output = vec4<ST>;
    #[inline]
    fn load(self) -> vec4<ST> {
        let _ = self;
        unimplemented!("no-op stub: only for type checking")
    }
}

impl<ST, C, L> TextureLoad for (texture_2d<ST>, vec2<C>, L) {
    type Output = vec4<ST>;
    #[inline]
    fn load(self) -> vec4<ST> {
        let _ = self;
        unimplemented!("no-op stub: only for type checking")
    }
}

impl<ST, A, C, L> TextureLoad for (texture_2d_array<ST>, vec2<C>, A, L) {
    type Output = vec4<ST>;
    #[inline]
    fn load(self) -> vec4<ST> {
        let _ = self;
        unimplemented!("no-op stub: only for type checking")
    }
}

/// 重载模拟入口:整体接收一个参数元组,rustc 按元组形状挑 impl。
#[inline]
pub fn textureLoad<A: TextureLoad>(args: A) -> A::Output {
    args.load()
}
