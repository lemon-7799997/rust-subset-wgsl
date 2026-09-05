# rust-subset-wgsl

把**合法 Rust 的受限子集**翻译成**合法 WGSL** 的编译期翻译器。

一份代码,两个用途:

1. **Rust 侧**:rustc 对 shader 代码做真实的类型检查(靠 `gpu` 桩库);
2. **GPU 侧**:`#[shader]` 宏在编译期把同一份代码翻译成 WGSL 字符串,交给 wgpu/naga 使用。

目标不是"任意 Rust → WGSL",而是定义一个小而可控的**双向子集**:Rust 语法是载体,WGSL 语义是终点。**只输出 WGSL 标准里存在的语法**,标准没有的东西一律编译报错,不做"看起来像"的假翻译。

---

## 这个翻译器做了什么

- **`gpu` 桩库(按需实现)**:定义 `vec2/3/4<T>`、`array<T,N>`、`texture_2d<T>`、`sampler`、常用数学函数等与 WGSL 同名的类型与函数。规则:**只实现"必须的功能"**——类型要有真实字段、容器要能存元素能下标、运算符要有语义(否则重放副本过不了 rustc);而 WGSL **数学/纹理内建函数保持 no-op**(`unimplemented!()`),因为 CPU 上永远不会执行 shader。
- **`gpu-macro`(`#[shader]`)**:吃掉一个 `mod`,做三件事:
  1. 把 AST 打印成 WGSL 文本(`pub const WGSL: &str`);
  2. 剥掉"翻译用装饰属性"后**重放原代码**,让 rustc 用桩库做类型检查;
  3. 子集外的语法 → 带源码位置的编译错误。
- **`src/main.rs`**:演示 shader + naga 校验单测。

## 大致执行流程

```
[你写] src/main.rs 里 #[shader] mod triangle { ... }   (普通合法 Rust + 桩类型)
   │
   │ cargo build / test
   ▼
[rustc] 语法解析 → 展开 #[shader] → 把整个 mod 的 token 交给宏
   │
   ▼
[gpu-macro 内]
   ① syn 解析成 AST(ItemMod)
   ② 收集模块内 struct 成员顺序(field_order,供字面量还原)
   ③ Ctx::print_* 逐 item 翻译成 WGSL 文本(属性映射/关键字改写/语句/表达式)
   ④ 剥掉装饰属性(#[group]/#[binding]/...)→ 原代码重放
   ⑤ 输出: pub const WGSL: &str = "..." + 剥干净的原代码
   │
   ▼
[rustc] 对重放副本做类型检查(桩库在此生效:类型写错当场报错)
   │
   ▼
[测试] naga::front::wgsl::parse_str + naga::valid::Validator 完整校验 WGSL
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
| `use gpu::*;` | 桩库导入,Rust-only | (不进 WGSL) |
| `#[group(0)] #[binding(0)] static u_scale: f32 = 1.0;` | 值类型 → uniform,初值丢弃 | `@group(0) @binding(0) var<uniform> u_scale: f32;` |
| `#[group(0)] #[binding(1)] static tex: texture_2d<f32> = texture_2d::new();` | **handle**(纹理)无地址空间,初值丢弃 | `@group(0) @binding(1) var tex: texture_2d<f32>;` |
| `#[group(0)] #[binding(2)] static smp: sampler = sampler;` | **handle**(采样器)无地址空间 | `@group(0) @binding(2) var smp: sampler;` |
| `#[group(0)] #[binding(4)] #[storage(read_write)] static mut buf: PositionBuffer = ...;` | **storage buffer**;可写用 `static mut`(Rust-only),初值丢弃 | `@group(0) @binding(4) var<storage, read_write> buf: PositionBuffer;` |
| `struct VsOut { ... }` | struct 定义;不允许泛型/tuple/空 | `struct VsOut { ... }` |
| `fn helper(...) -> ... { }` | 普通函数 | `fn helper(...) -> ... { }` |

> 一个 `#[shader] mod` 可以有多个入口(`@vertex` + `@fragment` + `@compute`),构成完整渲染/计算管线的配对。
> **`static mut` 只在 `#[storage(read_write)]` 下允许**,写入要包在 `unsafe {}` 里——这是 Rust-only 写法,翻译时 `unsafe` 块被**透明展开**,不产生任何 WGSL 痕迹。

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

> Rust 语法不允许给 `-> 返回类型` 挂属性,所以"返回值的装饰"约定写在 **fn 头**上,翻译时挪到返回类型前。
> `#[allow(...)]` 等普通属性不是装饰属性,宏会保留(仍属 Rust-only,不进 WGSL)。

### 3. struct

```rust
struct VsOut {
    #[builtin(position)]
    pos: vec4<f32>,
    #[location(0)]
    uv: vec2<f32>,
}
```

```wgsl
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
```

- 成员装饰属性 → WGSL 成员 `@装饰`。
- **构造**:Rust 命名字面量 → WGSL **位置构造器**(按 struct 声明顺序):

```rust
return VsOut { pos: vec4::<f32>(p.x, p.y, 0.0, 1.0), uv: p };
// → return VsOut(vec4<f32>(p.x, p.y, 0.0, 1.0), p);
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
| `unsafe { ... }` | (透明展开) | Rust-only 包装(如写 `static mut`),不产生任何 WGSL |

### 6. 表达式

| Rust 源 | WGSL | 说明 |
|---|---|---|
| `1.0` / `0` / `true` | 同 | 字面量 |
| `a + b` 等二元/一元 | 同(自动加括号) | `-`、`!` 支持 |
| `vec2::<f32>(x, y)` | `vec2<f32>(x, y)` | **turbofish 的 `::` 被吃掉**,元素类型显式写在代码里(翻译器不用猜) |
| `f32(vid)`(Rust 写 `vid as f32`) | `f32(vid)` | `as` 转换 → WGSL 转换构造器 |
| `p.x` | `p.x` | 字段访问(桩库字段名 = WGSL 名) |
| `arr[i]` | `arr[i]` | 索引 |
| `clamp(d, 0.0, 1.0)` 等 | 同 | 桩库数学函数,1:1 透传 |
| `textureSample(tex, smp, uv)` 等 | 同 | 桩库纹理函数(`textureSample` / `textureLoad` / `textureDimensions`),1:1 透传 |
| `VsOut { a: x, b: y }` | `VsOut(x, y)` | 见 struct 构造 |

### 7. 类型

- 标量 `f32` / `i32` / `u32`(Rust 原生类型,1:1)
- `vec2<T>` / `vec3<T>` / `vec4<T>`(桩库泛型结构体,尖括号语法两边一致)
- `array<T, N>`(桩库真容器:存 `[T; N]`,支持 `usize`/`u32` 下标——WGSL 数组下标是 `u32`,Rust 侧直接 `arr[i]` 不用 `as usize`)
- 模块内自定义 `struct`(不能泛型)
- **handle 类型**:`texture_2d<T>`、`sampler`。模块级声明**没有地址空间**:

```rust
// Rust                                     // WGSL
static tex: texture_2d<f32> = texture_2d::new();  // @group(0) @binding(1) var tex: texture_2d<f32>;
static smp: sampler = sampler;                     // @group(0) @binding(2) var smp: sampler;
```

## 不支持的语法(编译报错,不是悄悄跳过)

- `match` / 模式匹配 —— WGSL 没有(即使 `switch`,语义也与 Rust match 不同)
- `if` 当表达式(`let x = if ...`)—— WGSL 的 if 是语句
- 引用/指针:`&`、`&mut`、`self`、生命周期(`ptr` 不在子集里)
- 用户 struct 泛型、tuple struct、空 struct、`..base` 更新语法、结构体字面量引用模块外的 struct
- `for` 遍历非区间(如 vector)、`0..` 开区间
- `let` 无初值、`break 值`、带 label 的循环、裸块语句、`return` 出现在表达式位
- 多段路径(`a::b::c`)、方法调用、闭包、宏语句、`swizzle`(`.xy` 之类 Rust 语法根本没有)
- `array<T,N>` 的**字面量构造**(WGSL `array<f32,3>(1.0,2.0,3.0)` 没有对应的 Rust 语法)、**runtime 数组** `array<T>`(无 N)暂不支持——目前只有定长数组,出现在 static / struct 成员 / 下标读写里

## 项目结构

```
.
├── Cargo.toml            # workspace: 根 bin + gpu + gpu-macro
├── src/main.rs           # 演示 shader(#[shader] mod triangle)+ naga 校验单测
├── gpu/                  # 桩库(只实现"必须的功能",数学内建 no-op)
│   └── src/lib.rs        #   vec2/3/4<T>、array<T,N>、texture/sampler、数学函数
├── gpu-macro/            # proc-macro crate
│   └── src/lib.rs        #   #[shader] 翻译器(Ctx::print_*) + 透传属性宏
```

## 使用

```bash
# 看演示 shader 翻译出的 WGSL
cargo run

# 校验闭环:cargo test 会用 naga 解析并完整校验生成的 WGSL
cargo test
```

写自己的 shader:在任意 crate 里 `use gpu_macro::shader;`、`use gpu::*;`,用 `#[shader] mod` 包住 shader 代码,读 `your_mod::WGSL`。

## 设计原则与现状

- **桩库按需实现**:类型/容器/运算符等"必须的功能"要真实(否则重放副本过不了 rustc);WGSL 数学/纹理内建函数保持 no-op。
- **只做 WGSL 标准语法**;标准没有的语法在翻译时报错并指向源码位置。
- **两条护栏**:rustc 类型检查(重放副本)+ naga 完整校验(测试)。
- 现状:uniform / storage(读写)/ texture / sampler 模块级声明、定长 `array<T,N>`、struct、属性映射、if/else、loop/while/for、break/continue/return、unsafe 透明展开、多入口(vs+fs+compute)、构造器/cast/纹理函数透传都有;runtime 数组、UBO 布局属性(@size/@align)、矩阵、数组字面量还没做。

## 已知取舍

- 二元表达式统一加括号输出(如 `(p.x * a)`),保证优先级正确、WGSL 合法,只是不够"漂亮"。
- 数学函数桩签名故意很松(泛型不加约束),以后可用 trait 收紧,让明显错误的调用在 Rust 侧就报错。
- 桩库向量字段是 `pub`,`vec4 { x: .. }` 结构体字面量在 Rust 侧也"合法",但翻译器会把它当非法用法报错(请用 `vec4::<f32>(...)` 构造器)。
