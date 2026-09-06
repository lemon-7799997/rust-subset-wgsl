//! ============================================================================
//! 尝试 #7 —— 矩阵 + 相机 UBO + 模块级 const("MVP"真实案例):
//!   - `gpu` 桩库: vec2/3/4<T>、mat4x4<T>(列主序真容器,×vec4)、
//!     array<T>、texture/sampler handle、数学函数(no-op)
//!   - `gpu-macro` 的 #[shader]: ... + 模块级 const(翻译时挪到 WGSL 最前,
//!     因为 WGSL 要求先声明后使用)
//!   - 三入口:@vertex + @fragment + @compute;naga 完整校验
//! ============================================================================

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
// struct Camera { view_proj: mat4x4<f32>, }
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
    // 类型/函数都来自 gpu 桩库 → 这行编译后 rustc 会真检查下面所有类型
    use gpu::*;
    use gpu_macro::ConstDefault;

    // 模块级 const → WGSL const(翻译时自动挪到模块最前,先声明后使用)
    const GAMMA: f32 = 2.2;

    // static + 属性 => WGSL 模块级 var(见上面的期望产物)
    // #[allow] 不是装饰属性,宏会保留它;WGSL 全局变量约定就是小写命名
    #[allow(non_upper_case_globals)]
    #[group(0)]
    #[binding(0)]
    static u_scale: f32 = ConstDefault::DEFAULT;

    #[allow(non_upper_case_globals)]
    #[group(0)]
    #[binding(1)]
    static tex: texture_2d<f32> = ConstDefault::DEFAULT;

    #[allow(non_upper_case_globals)]
    #[group(0)]
    #[binding(2)]
    static smp: sampler = ConstDefault::DEFAULT;

    #[allow(non_upper_case_globals)]
    #[group(0)]
    #[binding(3)]
    static u_color: vec4<f32> = ConstDefault::DEFAULT;

    // storage buffer(compute 可写):
    //   #[storage(read_write)] + `static mut` = Rust 侧的可写全局
    //   (写入要 unsafe 块;翻译时 unsafe 透明剥掉,初值也丢弃)
    #[derive(ConstDefault)]
    struct PositionBuffer {
        pos: array<vec4<f32>>,
    }

    #[allow(non_upper_case_globals)]
    #[group(0)]
    #[binding(4)]
    #[storage(read_write)]
    static mut buf: PositionBuffer = ConstDefault::DEFAULT;

    // struct:字段上的装饰属性 → WGSL 成员 @装饰
    struct VsOut {
        #[builtin(position)]
        pos: vec4<f32>,
        #[location(0)]
        uv: vec2<f32>,
    }

    // 相机 UBO:uniform 里放 struct,struct 里放矩阵(默认布局即满足对齐)
    struct Camera {
        view_proj: mat4x4<f32>,
    }

    #[allow(non_upper_case_globals)]
    #[group(0)]
    #[binding(5)]
    static u_camera: Camera = Camera {
        view_proj: mat4x4::<f32>(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ),
    };

    #[vertex]
    fn vs_main(#[builtin(vertex_index)] vid: u32) -> VsOut {
        // let mut -> var;  `vid as f32` -> `f32(vid)`
        // vec2::<f32>(...) 的 `::<` 翻译时去掉 ::, 即 WGSL vec2<f32>(...)
        let mut p = vec2::<f32>(vid as f32, 0.0) * u_scale; // 向量 * 标量
        p = p - vec2::<f32>(0.5, 0.0); // 向量 - 向量

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
        let pos4 = vec4::<f32>(p.x, p.y * a, 0.0, 1.0);
        // MVP:先变换再输出(矩阵来自 Camera uniform,mat4x4 × vec4)
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
    fn fs_main(#[location(0)] uv: vec2<f32>) -> vec4<f32> {
        // 纹理采样 × uniform 颜色,再做伽马校正(模块级 const GAMMA)
        let c = textureSample(tex, smp, uv) * u_color;
        return pow(c, vec4::<f32>(GAMMA, GAMMA, GAMMA, 1.0));
    }

    // compute 入口:写 storage buffer(Rust 侧 static mut 要 unsafe,
    // 翻译时 unsafe 块透明展开成普通语句)
    #[compute]
    #[workgroup_size(8)]
    unsafe fn cs_main(#[builtin(global_invocation_id)] gid: vec3<u32>) {
        let i = gid.x; // u32 下标直接可用(桩库 array 实现了 Index<u32>)
        buf.pos[i] = vec4::<f32>(i as f32 * u_scale, 0.0, 0.0, 1.0);
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
