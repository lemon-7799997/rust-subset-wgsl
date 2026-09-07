// ============================================================================
// texture/sampler handle 家族 + 纹理函数桩 + TextureLoad 重载模拟
// WGSL 里 handle 声明在模块级但没有地址空间(不是 var<uniform>/<storage>):
//   @group(0) @binding(1) var tex: texture_2d<f32>;
//   @group(0) @binding(2) var smp: sampler;
// 翻译器看到这些 handle 类型时会跳过地址空间部分。
// ============================================================================

use core::marker::PhantomData;

use crate::{ConstDefault, IVec2, IVec3, UVec2, Vec2, Vec3, Vec4};

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

pub fn texture_sample_compare(
    t: texture_depth_2d,
    s: sampler_comparison,
    uv: Vec2,
    depth_ref: f32,
) -> f32 {
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
