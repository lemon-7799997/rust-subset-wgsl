//! # gpu —— WGSL 桩库(no-op, 只服务类型检查;向量/矩阵来自 glam)
//!
//! 全 glam 迁移之后,本 crate 里剩下的桩只有两类:
//!   - **WGSL-only 的名字**(翻译器看到时 1:1 照抄):
//!     texture/sampler handle、`array<T>`、数学自由函数;
//!   - **glam 再导出**(翻译器看到时按映射表译回 WGSL):
//!     `Vec2/Vec3/Vec4` → `vec2/vec3/vec4<f32>`、`Mat2/Mat3/Mat4` →
//!     `mat2x2/mat3x3/mat4x4<f32>`、`IVec*/UVec*` → `vecN<i32>/vecN<u32>`。
//!
//! 规矩没变:
//!   - 桩函数体永远不该被执行,数学/纹理桩全部 `unimplemented!()`;
//!   - 签名目前故意很松(泛型不加约束),以后再用 trait 收紧,
//!     让"错得离谱"的调用在 Rust 侧就报错。
//!   - glam 是**真类型**:重放副本对向量/矩阵运算是真实检查,比老桩更强;
//!     shader 代码 CPU 侧不执行,但共享 struct 的字段可以直接在 CPU 用。
#![allow(non_camel_case_types, non_snake_case)]

use core::marker::PhantomData;

// ============================================================================
// 向量/矩阵类型 = glam(单一来源,CPU/GPU 同构)
// 翻译器(gpu-macro)的 map_wgsl_type 认得下面这些名字,声明位置译回 WGSL 名。
// ============================================================================

pub use glam::{IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};

// ============================================================================
// 数学自由函数(no-op 桩;泛型签名,glam 向量/标量都能进)
// 名字 = WGSL 名,翻译器 1:1 透传。调用点写法与 WGSL 完全一致。
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
// 全 glam 迁移后,坐标/返回值类型直接用 glam(Vec2/Vec3/IVec3/UVec2...)。

pub fn textureSample<T>(t: texture_2d<T>, s: sampler, uv: Vec2) -> Vec4 {
    let _ = (t, s, uv);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_cube<T>(t: texture_cube<T>, s: sampler, uvw: Vec3) -> Vec4 {
    let _ = (t, s, uvw);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_array<T>(t: texture_2d_array<T>, s: sampler, uv: Vec2, layer: i32) -> Vec4 {
    let _ = (t, s, uv, layer);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_depth(t: texture_depth_2d, s: sampler, uv: Vec2) -> f32 {
    let _ = (t, s, uv);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_sample_compare(t: texture_depth_2d, s: sampler_comparison, uv: Vec2, depth_ref: f32) -> f32 {
    let _ = (t, s, uv, depth_ref);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_load_cube<T>(t: texture_cube<T>, coords: IVec3, level: i32) -> Vec4 {
    let _ = (t, coords, level);
    unimplemented!("no-op stub: only for type checking")
}

pub fn textureDimensions<T>(t: texture_2d<T>, level: i32) -> UVec2 {
    let _ = (t, level);
    unimplemented!("no-op stub: only for type checking")
}

pub fn texture_dimensions_array<T>(t: texture_2d_array<T>, level: i32) -> UVec2 {
    let _ = (t, level);
    unimplemented!("no-op stub: only for type checking")
}

// ============================================================================
// array<T>:假容器(桩库规则:只实现"必须的功能")
// 只放一个 phantom 元素,Index/IndexMut 忽略下标——目的纯粹是让重放副本
// 能过 rustc 类型检查;真正的数组是 WGSL 侧的 storage buffer 成员。
// WGSL 数组下标是 u32/i32,所以支持 usize/u32 下标,让源写法贴近 WGSL。
// 元素类型可以是 glam 向量(如 array<Vec4>),只要 T: ConstDefault。
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

// ============================================================================
// 重载模拟:用「自由函数接收元组 + trait 分发」模拟 textureLoad 重载
// ============================================================================
// WGSL 的 textureLoad 按纹理类型有多个签名,而 Rust 没有函数重载。
// 这里让 textureLoad 整体接收一个元组,trait 按元组形状选实现:
//
//   textureLoad((texture_1d<ST>,       C,    L))
//   textureLoad((texture_2d<ST>,       IVec2, L))
//   textureLoad((texture_2d_array<ST>, IVec2, A, L))
//
// 调用点写法与 WGSL 名完全一致(只是参数变成"一整包元组");
// rustc 靠 A: TextureLoad 约束检查元组形状/类型;
// 翻译器(gpu-macro)看到 textureLoad((…)) 会把元组摊平成 textureLoad(…)。

pub trait TextureLoad {
    /// 返回类型。全 glam 迁移后采样结果统一是 f32 四元组 Vec4
    /// (WGSL textureLoad 的元素类型实际由纹理格式决定,这里只支持 f32 系)。
    type Output;
    fn load(self) -> Self::Output;
}

impl<ST, C, L> TextureLoad for (texture_1d<ST>, C, L) {
    type Output = Vec4;
    #[inline]
    fn load(self) -> Vec4 {
        let _ = self;
        unimplemented!("no-op stub: only for type checking")
    }
}

impl<ST, L> TextureLoad for (texture_2d<ST>, IVec2, L) {
    type Output = Vec4;
    #[inline]
    fn load(self) -> Vec4 {
        let _ = self;
        unimplemented!("no-op stub: only for type checking")
    }
}

impl<ST, A, L> TextureLoad for (texture_2d_array<ST>, IVec2, A, L) {
    type Output = Vec4;
    #[inline]
    fn load(self) -> Vec4 {
        let _ = self;
        unimplemented!("no-op stub: only for type checking")
    }
}

/// 重载模拟入口:整体接收一个参数元组,rustc 按元组形状挑 impl。
#[inline]
pub fn textureLoad<A: TextureLoad>(args: A) -> A::Output {
    args.load()
}
