# rust-subset-wgsl

把**合法 Rust 的受限子集**翻译成**合法 WGSL** 的编译期翻译器。

一份代码,两个用途:

1. **Rust 侧**:rustc 对 shader 代码做真实的类型检查(向量/矩阵是 **glam 真类型**,其余靠 `gpu` 桩库);
2. **GPU 侧**:`#[shader]` 宏在编译期把同一份代码翻译成 WGSL 字符串,交给 wgpu/naga 使用。

目标不是"任意 Rust → WGSL",而是定义一个小而可控的**双向子集**:Rust 语法是载体,WGSL 语义是终点。**只输出 WGSL 标准里存在的语法**,标准没有的东西一律编译报错,不做"看起来像"的假翻译。

---

## 这个翻译器做了什么

- **`gpu` 桩库(按需实现)**:**向量/矩阵直接 re-export glam**(`Vec2/3/4`、`Mat2/3/4`、`IVec*/UVec*`),翻译器把 glam 类型名/构造调用译回 WGSL;其余 WGSL-only 的东西仍以"名字 = WGSL 名"的桩形式提供:`array<T>` 假容器、`texture_2d<T>`/`sampler` 等 handle、数学/纹理自由函数(no-op,CPU 上永不执行 shader)。
- **`gpu-macro`(`#[shader]`)**:吃掉一个 `mod`,做四件事:
  1. 把 AST 打印成 WGSL 文本(`pub const WGSL: &str`);
  2. **自校验**:宏展开时直接用 naga 解析 + 完整校验产物,不合法 → 编译错误(feature `self-validate`,默认开,见下);
  3. 剥掉"翻译用装饰属性"后**重放原代码**,让 rustc 做类型检查;
  4. 子集外的语法 → 带源码位置的编译错误。
- **`src/main.rs`**:演示 shader + naga 校验单测。

## 大致执行流程

```
[你写] src/main.rs 里 #[shader] mod triangle { ... }   (普通合法 Rust + glam/桩类型)
   │
   │ cargo build / test
   ▼
[rustc] 语法解析 → 展开 #[shader] → 把整个 mod 的 token 交给宏
   │
   ▼
[gpu-macro 内]
   ① syn 解析成 AST(ItemMod)
   ② 收集模块内 struct 成员顺序(field_order,供字面量还原)
   ③ Ctx::print_* 逐 item 翻译成 WGSL 文本(glam 名/构造映射、属性映射、语句/表达式)
   ④ 自校验: naga parse + Validator 校验产物,失败 → 编译错误
   ⑤ 剥掉装饰属性(#[group]/#[binding]/...)→ 原代码重放
   ⑥ 输出: pub const WGSL: &str = "..." + 剥干净的原代码
   │
   ▼
[rustc] 对重放副本做类型检查(向量/矩阵 = glam 真语义,其余靠桩库)
   │
   ▼
[测试] 演示单测再跑一遍 naga(兼带入口 stage 断言)
   │
   ▼
[运行时] 取 triangle::WGSL 字符串喂给 wgpu 建 ShaderModule
```

设计要点:**单一来源**——shader 只写一遍;类型正确性由 rustc 保证,语法合法性由 naga 保证,翻译器夹在中间只做机械映射。

## 支持的语法(清单)

约定:Rust 源里 `#[xxx]` 是"翻译用装饰属性",翻译后变成 WGSL 的 `@xxx`;它们不是 Rust 原生属性,会被宏剥掉(所以不会出现在重放副本里)。

### 1. 模块级

| Rust 源 | 说明 | WGSL |
|---|---|---|
| `use gpu::*;` | 桩库导入(含 glam 类型再导出),Rust-only | (不进 WGSL) |
| `const GAMMA: f32 = 2.2;` | 模块级 const(翻译时自动挪到模块最前,满足 WGSL"先声明后使用") | `const GAMMA: f32 = 2.2;` |
| `#[group(0)] #[binding(0)] static u_scale: f32 = ...;` | 值类型 → uniform,初值丢弃 | `@group(0) @binding(0) var<uniform> u_scale: f32;` |
| `#[group(0)] #[binding(3)] static u_color: Vec4 = ...;` | glam 向量 uniform(类型映射) | `@group(0) @binding(3) var<uniform> u_color: vec4<f32>;` |
| `#[group(0)] #[binding(5)] static u_cam: Camera = ...;` | struct uniform(Camera 成员含 Mat4/Vec3) | `@group(0) @binding(5) var<uniform> u_cam: Camera;` |
| `#[group(0)] #[binding(1)] static tex: texture_2d<f32> = ...;` | **handle**(纹理)无地址空间,初值丢弃 | `@group(0) @binding(1) var tex: texture_2d<f32>;` |
| `#[group(0)] #[binding(2)] static smp: sampler = sampler;` | **handle**(采样器)无地址空间 | `@group(0) @binding(2) var smp: sampler;` |
| `#[group(0)] #[binding(4)] #[storage(read_write)] static mut buf: PositionBuffer = ...;` | **storage buffer**;可写用 `static mut`(Rust-only),初值丢弃 | `@group(0) @binding(4) var<storage, read_write> buf: PositionBuffer;` |
| `struct VsOut { ... }` | struct 定义;不允许泛型/tuple/空 | `struct VsOut { ... }` |
| `fn helper(...) -> ... { }` | 普通函数 | `fn helper(...) -> ... { }` |

> 一个 `#[shader] mod` 可以有多个入口(`@vertex` + `@fragment` + `@compute`),构成完整渲染/计算管线的配对。
> **`static mut` 只在 `#[storage(read_write)]` 下允许**,写入的 fn 声明为 `unsafe fn`(或写入段包 `unsafe {}`)——这是 Rust-only 写法,翻译时 `unsafe` 被**透明剥掉**,不产生任何 WGSL 痕迹。
> **static 的初值只用于 Rust 侧类型检查,翻译时整体丢弃**——所以那里可以放心用 `Mat4::IDENTITY`/`Vec3::ZERO` 这类 glam 常量(它们不会进 WGSL)。
> 模块内声明注意**源顺序**:WGSL 要求 struct/static/const 先声明后使用,翻译按源顺序输出(const 例外,会被提前)。

### 2. 装饰属性(映射表)

| Rust 属性 | 位置 | WGSL |
|---|---|---|
| `#[group(0)]` / `#[binding(0)]` | static | `@group(0)` / `@binding(0)` |
| `#[storage]` / `#[storage(read)]` / `#[storage(read_write)]` | static | 地址空间 `<storage>` / `<storage, read_write>`(不是 @ 装饰) |
| `#[vertex]` / `#[fragment]` / `#[compute]` | fn | `@vertex` / `@fragment` / `@compute` |
| `#[workgroup_size(8, 8, 1)]` | compute fn | `@workgroup_size(8, 8, 1)` |
| `#[builtin(position)]` | fn(装饰**返回值**) | `-> @builtin(position) vec4<f32>` |
| `#[builtin(vertex_index)]` | 参数 | `@builtin(vertex_index) vid: u32` |
| `#[location(0)]` | 参数 / struct 成员 / fn(返回值) | `@location(0)` |
| `#[interpolate(flat)]` | 参数 / struct 成员 | `@interpolate(flat)` |
| `#[align(16)]` / `#[size(16)]` | struct 成员(布局) | 成员 `@align(16)` / `@size(16)` |

> Rust 语法不允许给 `-> 返回类型` 挂属性,所以"返回值的装饰"约定写在 **fn 头**上,翻译时挪到返回类型前。
> `#[allow(...)]` 等普通属性不是装饰属性,宏会保留(仍属 Rust-only,不进 WGSL)。

### 3. struct

```rust
struct VsOut {
    #[builtin(position)]
    pos: Vec4,            // Rust 侧 = glam
    #[location(0)]
    uv: Vec2,
}
```

```wgsl
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
```

- 成员装饰属性 → WGSL 成员 `@装饰`;成员类型若为 glam(`Vec4`/`Mat4`/`UVec3`…),按映射表译回 `vec4<f32>`/`mat4x4<f32>`/`vec3<u32>`。
- **构造**:Rust 命名字面量 → WGSL **位置构造器**(按 struct 声明顺序):

```rust
return VsOut { pos: u_camera.view_proj * Vec4::new(p.x, p.y, 0.0, 1.0), uv: p };
// → return VsOut(u_camera.view_proj * vec4<f32>(p.x, p.y, 0.0, 1.0), p);
```

成员名/个数由 rustc 保证(重放副本会检查),翻译器只做"名字 → 声明顺序"的映射。

### 4. 函数

```rust
#[vertex]
fn vs_main(#[builtin(vertex_index)] vid: u32) -> VsOut {
    ...
}
```

- fn 级 stage 属性放 fn 前;返回值装饰挪到 `->` 后;参数属性原位保留。
- 函数体缩进/大括号结构原样保留。

### 5. 语句

| Rust 源 | WGSL | 说明 |
|---|---|---|
| `let x = e;` | `let x = e;` | 不可变绑定(与 WGSL `let` 一致) |
| `let mut x = e;` | `var x = e;` | 可变局部(必须带初值,WGSL 不允许无初值) |
| `x = e;` | `x = e;` | 赋值 |
| `if c { } else if c2 { } else { }` | 同构 | 含空分支 |
| `loop { ... }` | `loop { ... }` | 配 `break` / `continue` 使用 |
| `while c { ... }` | `while c { ... }` | |
| `for k in 0..N { ... }` | `for (var k = 0; k < N; k = k + 1) { ... }` | 区间重写;`0..=N` → `<=` |
| `break;` / `continue;` | 同构 | `break` 不允许带值 |
| `return e;` / `return;` | 同构 | |
| `unsafe { ... }` / `unsafe fn` | (透明剥掉) | Rust-only(写 `static mut` 需要),不产生任何 WGSL |

### 6. 表达式

| Rust 源 | WGSL | 说明 |
|---|---|---|
| `1.0` / `0` / `true` | 同 | 字面量 |
| `a + b` 等二元/一元 | 同(自动加括号) | `-`、`!` 支持;glam 向量的运算符(±、×标量、mat×vec/mat×mat、字段 x/y/z/w)都是真实现,文本 1:1 透传 |
| `Vec2::new(x, y)` / `Vec3::new(x, y, z)` / `Vec4::new(x, y, z, w)` | `vec2<f32>(...)` 等 | **glam 构造映射**(元素类型是 f32) |
| `Vec4::splat(v)` | `vec4<f32>(v, v, v, v)` | 单参 splat 展开成 dim 个实参,最稳 |
| `IVec2::new(x, y)` / `UVec3::new(x, y, z)` | `vec2<i32>(...)` / `vec3<u32>(...)` | i32/u32 向量构造 |
| `Mat2::from_cols(a, b)` / `Mat3::from_cols(a, b, c)` / `Mat4::from_cols(a, b, c, d)` | `mat2x2<f32>(a, b)` 等 | 按 2/3/4 个列向量构造(WGSL 支持按列传列向量) |
| `u_cam.view_proj * pos4` | 同 | `Mat4 × Vec4`(glam 运算符) |
| `f32(vid)`(Rust 写 `vid as f32`) | `f32(vid)` | `as` 转换 → WGSL 转换构造器 |
| `p.x` | `p.x` | 字段访问(glam 字段名 = WGSL 分量名) |
| `arr[i]` | `arr[i]` | 索引 |
| `clamp(d, 0.0, 1.0)` / `length(p)` 等 | 同 | 桩库数学自由函数,泛型签名,标量与 glam 向量都能进;1:1 透传 |
| `textureSample(tex, smp, uv)` / `texture_sample_cube(...)` 等 | 同(改名表映射) | **Rust 无重载**:同名不同签名的 WGSL 内建拆成不同 Rust 名,翻译时映射回 `textureSample`/`textureSampleCompare`/`textureLoad` 等;坐标/返回值已是 glam(`Vec2`/`Vec4`/`UVec2`…) |
| `VsOut { a: x, b: y }` | `VsOut(x, y)` | 见 struct 构造 |

### 7. 类型

- 标量 `f32` / `i32` / `u32`(Rust 原生类型,1:1)
- **glam 向量/矩阵(经 `gpu` 再导出,真类型)**:

| Rust(源码里写) | WGSL |
|---|---|
| `Vec2` / `Vec3` / `Vec4` | `vec2<f32>` / `vec3<f32>` / `vec4<f32>` |
| `IVec2/3/4`、`UVec2/3/4` | `vecN<i32>`、`vecN<u32>` |
| `Mat2` / `Mat3` / `Mat4` | `mat2x2<f32>` / `mat3x3<f32>` / `mat4x4<f32>` |

- `array<T>`(runtime 数组形态,用于 storage buffer 成员;内部是"假容器",Index 忽略下标只为过类型检查;元素可以是 glam 类型,如 `array<Vec4>`)
- 模块内自定义 `struct`(不能泛型)
- **handle 类型**:`texture_2d<T>` / `texture_2d_array<T>` / `texture_cube<T>` / `texture_depth_2d` / `sampler` / `sampler_comparison`。模块级声明**没有地址空间**:

```rust
// Rust                                     // WGSL
static tex: texture_2d<f32> = ...;          // @group(0) @binding(1) var tex: texture_2d<f32>;
static smp: sampler = sampler;              // @group(0) @binding(2) var smp: sampler;
```

> 共享 struct(要 CPU 侧也用的布局 struct)就声明在 shader mod 内并标 `pub`:字段用 glam 类型,
> CPU 侧 `use` 同一个类型即可——WGSL 声明自动生成,字段只写一处。

## 不支持的语法(编译报错,不是悄悄跳过)

- `match` / 模式匹配 —— WGSL 没有(即使 `switch`,语义也与 Rust match 不同)
- `if` 当表达式(`let x = if ...`)—— WGSL 的 if 是语句
- 引用/指针:`&`、`&mut`、`self`、生命周期(`ptr` 不在子集里)
- 用户 struct 泛型、tuple struct、空 struct、`..base` 更新语法、结构体字面量引用模块外的 struct
- `for` 遍历非区间(如 vector)、`0..` 开区间
- `let` 无初值、`break 值`、带 label 的循环、裸块语句、`return` 出现在表达式位
- 多段路径(`a::b::c`)、方法调用、闭包、宏语句、`swizzle`(`.xy` 之类 Rust 语法根本没有)
- **glam 常量(`Vec4::ZERO` 等)与 glam 方法调用**(`.normalize()`/`.dot()` 等)出现在**会翻译的表达式**里(static 初值例外,整体丢弃不翻译)——目前的构造白名单只有 `new` / `splat` / `MatN::from_cols`
- `array<T>` 的**字面量构造**(WGSL `array<f32,3>(1.0,2.0,3.0)` 没有对应的 Rust 语法)、**定长数组**(含 uniform 里的数组)暂不支持——目前 runtime 形态 `array<T>` 只出现在 storage buffer 成员里

## 项目结构

```
.
├── Cargo.toml            # workspace: 根 bin + gpu + gpu-macro
├── src/main.rs           # 演示 shader(#[shader] mod triangle)+ naga 校验单测
├── gpu/                  # 桩库:glam 再导出 + handle/array/数学桩(数学内建 no-op)
│   └── src/lib.rs        #   pub use glam::{Vec2..Mat4..}; texture/sampler; array<T>; ConstDefault; 数学函数
├── gpu-macro/            # proc-macro crate
│   └── src/lib.rs        #   #[shader] 翻译器(glam 映射 + Ctx::print_*)+ 透传宏 + ConstDefault derive + naga 自校验
```

## 使用

```bash
# 看演示 shader 翻译出的 WGSL
cargo run

# 校验闭环:cargo test 会用 naga 解析并完整校验生成的 WGSL
cargo test
```

写自己的 shader:在任意 crate 里 `use gpu_macro::shader;`、`use gpu::*;`,用 `#[shader] mod` 包住 shader 代码,读 `your_mod::WGSL`。

### 关闭宏内自校验

`gpu-macro` 默认在宏展开期做 naga 自校验(`self-validate` feature,默认开):任何 `#[shader]` 编译时即被 naga 检查,生成的 WGSL 不合法会直接编译报错。想关闭(如为编译速度、或已用外部校验)时,依赖写成:

```toml
gpu-macro = { path = "..", default-features = false }
```

## 设计原则与现状

- **桩库按需实现**:向量/矩阵直接来自 glam(真类型、真运算,CPU 侧共享 struct 也能直接用);array/handle/数学桩保持 no-op、名字 = WGSL 名。
- **只做 WGSL 标准语法**;标准没有的语法在翻译时报错并指向源码位置。
- **三层护栏**:rustc 类型检查(重放副本,glam 部分是真语义)+ 宏展开期 naga 自校验(默认开)+ 演示单测的 naga 校验。
- **暂未实现(备选方向,以后再说)**:模块声明顺序的自动检查/调整(struct/static 目前按源顺序输出、const 自动提前)、binding 号/变量名冲突检测、重复绑定检查。设计初衷是"rust 风味的 WGSL 编写体验":用户代码以 WGSL 正确性为优先,翻译器保证生成的 WGSL 合法(rustc 类型检查 + 宏内 naga 自校验);这类"替你纠错"的兜底检查不是优先项,需要时再加。
- 现状:glam 向量/矩阵(Vec2/3/4、Mat2/3/4、IVec*/UVec*,声明/构造映射)、uniform / storage(读写)/ texture / sampler 模块级声明、模块级 const、struct、属性映射、if/else、loop/while/for、break/continue/return、unsafe 透明(块/fn)、多入口(vs+fs+compute)、构造器/cast/纹理函数透传、矩阵×向量、UBO 布局属性(@size/@align)、纹理家族(cube/2d_array/depth + compare 采样器)都有;定长/字面量数组、swizzle、glam 方法风(shader 内方法调用)还没做。

## 已知取舍

- 二元表达式统一加括号输出(如 `(p.x * a)`),保证优先级正确、WGSL 合法,只是不够"漂亮"。
- 数学函数桩签名故意很松(泛型不加约束),以后可用 trait 收紧,让明显错误的调用在 Rust 侧就报错。
- glam 类型映射集中在翻译器一张表(`glam_type_shape`):加类型/构造时改一处;glam 常量与方法调用的支持(方法风)是后续方向,目前报错拒绝。
- **重载模拟(实验)**:Rust 无函数重载,WGSL 同名不同签名内建有两种处理:改名表(如 `texture_sample_cube` → `textureSample`)与「自由函数接收元组 + trait 分发」(见 `gpu` 的 `TextureLoad`:`textureLoad((tex, coords, level))` → WGSL `textureLoad(tex, coords, level)`)。trait 按元组形状选签名并约束参数类型,调用名与 WGSL 完全一致;代价是参数包了一层元组、裸字面量可能让推断卡住(建议用有类型的参数);两条路先都留着,看手感再收敛。
- glam 向量字段是 `pub`,`Vec4 { x: .. }` 结构体字面量在 Rust 侧"合法",但翻译器会把它当非法用法报错(请用 `Vec4::new(...)` 构造器)。
- **上传布局提醒**:共享 struct 的 Rust 内存布局 ≠ WGSL uniform 默认布局(WGSL 里 vec4 对齐 16、嵌套 struct 后面成员的 roundUp(16) 规则……)。需要整块字节上传时,CPU 侧布局要以 WGSL 默认布局为准(repr(C) + 显式 padding,或用 encase 之类的布局工具),别直接 bytemuck 拷未对齐的 Rust struct。
