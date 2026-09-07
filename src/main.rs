//! ============================================================================
//! 尝试 #8 —— 全 glam 迁移:
//!   - `gpu` 桩库不再定义 vec/mat,**向量/矩阵 = glam(Vec2/3/4、Mat2/3/4、
//!     IVec*/UVec*)**,翻译器按映射表把类型名/构造调用译回 WGSL
//!     (`Vec4` → `vec4<f32>`、`Vec4::new(..)` → `vec4<f32>(..)`、
//!     `Mat4::from_cols(..)` → `mat4x4<f32>(列向量…)`);
//!   - texture/sampler handle、array<T>、数学自由函数仍是桩(名字 = WGSL 名);
//!   - 三入口:@vertex + @fragment + @compute;naga 完整校验
//! ============================================================================
#![allow(non_upper_case_globals)]

use gpu_macro::shader;

// ---- 期望生成的 WGSL(翻译产物,节选) ----
// const GAMMA: f32 = 2.2;                        // ← 模块级 const,自动排最前
//
// @group(0) @binding(0) var<uniform> u_scale: f32;
// @group(0) @binding(1) var tex: texture_2d<f32>;
// @group(0) @binding(2) var smp: sampler;
// @group(0) @binding(3) var<uniform> u_color: vec4<f32>;
//
// struct PositionBuffer { pos: array<vec4<f32>>, }
// @group(0) @binding(4) var<storage, read_write> buf: PositionBuffer;
//
// struct Camera { view_proj: mat4x4<f32>, @align(16) @size(16) tint: vec3<f32> }
// @group(0) @binding(5) var<uniform> u_camera: Camera;
//
// struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
//
// @vertex fn vs_main(...) -> VsOut {
//     ...
//     return VsOut(u_camera.view_proj * vec4<f32>(p.x, p.y, 0.0, 1.0), p);
// }
//
// @fragment fn fs_main(...) -> @location(0) vec4<f32> {
//     return pow(c * u_color, vec4<f32>(GAMMA, GAMMA, GAMMA, 1.0));  // 伽马
// }

#[shader]
mod triangle {

    // 类型/函数来自 gpu 桩库:glam 的向量矩阵经 gpu 再导出,`use gpu::*` 一次引入;
    // 编译后 rustc 会对下面所有类型做真检查(向量/矩阵运算是 glam 的真实语义)
    use gpu::{textureLoad, *};
    use gpu_macro::ConstDefault;

    // 模块级 const → WGSL const(翻译时自动挪到模块最前,先声明后使用)
    const GAMMA: f32 = 2.2;

    // static + 属性 => WGSL 模块级 var(见上面的期望产物)
    // #[allow] 不是装饰属性,宏会保留它;WGSL 全局变量约定就是小写命名
    #[group(0)]
    #[binding(0)]
    static u_scale: f32 = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(1)]
    static tex: texture_2d<f32> = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(2)]
    static smp: sampler = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(3)]
    static u_color: Vec4 = ConstDefault::DEFAULT;

    // storage buffer(compute 可写):
    //   #[storage(read_write)] + `static mut` = Rust 侧的可写全局
    //   (写入要 unsafe 块;翻译时 unsafe 透明剥掉,初值也丢弃)
    #[derive(ConstDefault)]
    struct PositionBuffer {
        pos: array<Vec4>,
    }

    #[group(0)]
    #[binding(4)]
    #[storage(read_write)]
    static mut buf: PositionBuffer = ConstDefault::DEFAULT;

    // struct:字段上的装饰属性 → WGSL 成员 @装饰
    struct VsOut {
        #[builtin(position)]
        pos: Vec4,
        #[location(0)]
        uv: Vec2,
    }

    // 相机 UBO:uniform 里放 struct,struct 里放矩阵(默认布局即满足对齐)
    // 布局属性 #[align(N)]/#[size(N)] → 成员 @align(N)/@size(N)
    struct Camera {
        view_proj: Mat4,
        #[align(16)]
        #[size(16)] // vec3 默认 12B,@size(16) 补足到 16(对齐 16 的倍数)
        tint: Vec3,
    }

    #[group(0)]
    #[binding(5)]
    // static 初值只用于 Rust 侧类型检查(翻译时丢弃),所以这里可以直接用
    // 构造宏(mat4x4f! 16 标量列主序、vec3f! 单参 splat),不会进 WGSL
    static u_camera: Camera = Camera {
        view_proj: mat4x4f!(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ),
        tint: vec3f!(0.0),
    };

    // 纹理家族:cube / 2d_array / depth + compare 采样器(handle,无地址空间)
    #[group(0)]
    #[binding(6)]
    static tex_cube: texture_cube<f32> = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(7)]
    static tex_arr: texture_2d_array<f32> = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(8)]
    static depth_tex: texture_depth_2d = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(9)]
    static smp_cmp: sampler_comparison = ConstDefault::DEFAULT;

    #[group(0)]
    #[binding(10)]
    static tex_1d: texture_1d<f32> = ConstDefault::DEFAULT;

    #[vertex]
    fn vs_main(#[builtin(vertex_index)] vid: u32) -> VsOut {
        // let mut -> var;  `vid as f32` -> `f32(vid)`
        // vec2f!(x, y) -> WGSL vec2<f32>(x, y)(构造宏 → WGSL 构造器)
        let mut p = vec2f!(vid as f32, 0.0) * u_scale; // 向量 * 标量
        p = p - vec2f!(0.5, 0.0); // 向量 - 向量

        // if / else if / else
        let mut sign = 1.0;
        if p.x > 0.0 {
            sign = -1.0;
        } else if p.x < -0.5 {
            sign = 0.0;
        } else {
            // 保持 1.0(空 else 分支也允许)
        }

        // loop + break/continue
        let mut i = 0.0;
        loop {
            i = i + 1.0;
            if i > 3.0 {
                break;
            }
            if i < 0.0 {
                continue;
            }
            p.x = p.x + sign * 0.1;
        }

        // while
        let mut j = 0.0;
        while j < 2.0 {
            p.y = p.y + 0.05;
            j = j + 1.0;
        }

        // for 0..N(Rust for-in 区间 → WGSL for 头)
        for k in 0..3 {
            p.y = p.y + k as f32 * 0.01;
        }

        let d = length(p); // 数学函数桩: length(vec) -> f32
        let a = clamp(d, 0.0, 1.0);
        let pos4 = vec4f!(p.x, p.y * a, 0.0, 1.0);
        // MVP:先变换再输出(矩阵来自 Camera uniform,Mat4 × Vec4)
        // struct 字面量(按声明顺序)→ WGSL 位置构造器 VsOut(pos, uv)
        return VsOut {
            pos: u_camera.view_proj * pos4,
            uv: p,
        };
    }

    // 片元入口:参数 @location(0) uv 与顶点输出 VsOut.uv 配对
    // fn 级 #[location(0)] = 装饰返回值(→ @location(0) vec4<f32>)
    #[fragment]
    #[location(0)]
    fn fs_main(#[location(0)] uv: Vec2) -> Vec4 {
        // 2D 采样 × uniform 颜色(textureSample 返回 Vec4)
        let c = textureSample(tex, smp, uv) * u_color;
        // 纹理家族:cube / 2d_array / depth+compare(Rust 名 → WGSL 名走改名表)
        let env = texture_sample_cube(tex_cube, smp, vec3f!(uv.x, uv.y, 1.0));
        let arr = texture_sample_array(tex_arr, smp, uv, 0);
        let sh = texture_sample_compare(depth_tex, smp_cmp, uv, 0.5);
        let lit = c * env * arr * vec4f!(sh, sh, sh, 1.0);
        return pow(lit, vec4f!(GAMMA, GAMMA, GAMMA, 1.0));
    }

    // 非入口辅助函数:同样会被翻译并校验(mat3x3f! 按列传列构造/对角矩阵、
    // 矩阵×矩阵、矩阵×向量——WGSL 的 mat3x3<f32>(vec3<f32>…) 同构)
    fn transform_tangent(p: Vec3) -> Vec3 {
        let basis = mat3x3f!(
            vec3f!(1.0, 0.0, 0.0), //
            vec3f!(0.0, 1.0, 0.0), //
            vec3f!(0.0, 0.0, 1.0), //
        );
        let rot = mat3x3f!(
            vec3f!(0.0, -1.0, 0.0), //
            vec3f!(1.0, 0.0, 0.0),  //
            vec3f!(0.0, 0.0, 1.0),  //
        );
        let scale = mat3x3f!(2.0); // 单标量 → 对角矩阵(WGSL 语义)
        return (basis * rot) * (scale * p); // 矩阵×矩阵 再 矩阵×向量
    }

    // 构造宏冒烟函数(非入口,同样会被翻译并 naga 校验):
    // vec2f! 单参 splat、mat2x2f! 对角、mat4x4f! 16 标量、vec4f! 全填/splat
    fn macro_smoke(v: f32) -> f32 {
        let a = vec2f!(v); // splat → vec2<f32>(v, v)
        let m = mat2x2f!(v); // 对角 → mat2x2<f32>(v, 0.0, 0.0, v)
        let c = m * a;
        let id4 = mat4x4f!(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        );
        let d = id4 * vec4f!(v, v, v, v);
        let e = id4 * vec4f!(v); // splat → vec4<f32>(v, v, v, v)
        return c.x + c.y + d.x + e.y;
    }

    // 元组+trait 重载模拟实验(textureLoad 接收整包元组,三种签名):
    // Rust 写 textureLoad((tex, coord, level)),翻译成 WGSL textureLoad(...)
    // 全 glam 迁移后坐标类型是 IVec2(→ vec2<i32>)
    fn probe_load(coord: IVec2, level: i32, layer: i32) -> f32 {
        let a = textureLoad((tex, coord, level)); // texture_2d<f32>, IVec2, i32
        let b = textureLoad((tex_arr, coord, layer, level)); // 2d_array 多一层
        let c = textureLoad((tex_1d, coord.x, level)); // texture_1d, i32, i32
        return a.x + b.x + c.x;
    }

    // compute 入口:写 storage buffer(Rust 侧 static mut 要 unsafe,
    // 翻译时 unsafe 透明剥掉)
    #[compute]
    #[workgroup_size(8)]
    unsafe fn cs_main(#[builtin(global_invocation_id)] gid: UVec3) {
        let i = gid.x; // u32 下标直接可用(桩库 array 实现了 Index<u32>)
                       // mat2x2f! 按两列向量构造旋转(u_scale 当角度),
                       // 顺带覆盖 mat2x2 × vec2 与 cos/sin
        let ang = u_scale;
        let rot = mat2x2f!(
            vec2f!(cos(ang), sin(ang)),  //
            vec2f!(-sin(ang), cos(ang)), //
        );
        let p2 = rot * vec2f!(i as f32, 0.0);
        buf.pos[i] = vec4f!(p2.x, p2.y, 0.0, 1.0);
    }
}

fn main() {
    // 宏吐出的 WGSL(现在是真翻译产物)
    println!("{}", triangle::WGSL);
}

#[cfg(test)]
mod tests {
    use super::*;
    use naga::valid::{Capabilities, ValidationFlags, Validator};

    /// 校验闭环:生成的 WGSL 必须能被 naga 解析并通过完整校验
    #[test]
    fn generated_wgsl_is_valid() {
        let module = naga::front::wgsl::parse_str(triangle::WGSL).expect("生成的 WGSL 语法不合法!");
        // 深层校验:类型推断、struct 布局、地址空间、循环/continue 规则等
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        validator
            .validate(&module)
            .expect("生成的 WGSL 未通过 naga 完整校验!");
        let stages: Vec<naga::ShaderStage> =
            module.entry_points.iter().map(|ep| ep.stage).collect();
        assert!(
            stages.contains(&naga::ShaderStage::Vertex),
            "缺少 @vertex 入口点"
        );
        assert!(
            stages.contains(&naga::ShaderStage::Fragment),
            "缺少 @fragment 入口点"
        );
        assert!(
            stages.contains(&naga::ShaderStage::Compute),
            "缺少 @compute 入口点"
        );
    }
}
