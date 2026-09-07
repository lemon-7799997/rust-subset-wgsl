# agents.md

给在本仓库工作的 AI agent(以及人类协作者)的说明。先读 README.md 了解项目定位;这份文件讲**改代码时必须守住的规则**和**加功能的标准动作**。

## 项目一句话

`#[shader]` 属性宏把"合法 Rust 受限子集"翻译成"合法 WGSL";向量/矩阵直接用 **glam**(经 `gpu` 再导出),`gpu` 是**按需实现**的桩库(handle/array/数学桩,数学内建 no-op),让 rustc 对 shader 代码做类型检查;naga 单测保证产物合法。

## 仓库布局

| 路径 | 内容 | 改它之前要知道 |
|---|---|---|
| `src/main.rs` | 演示 shader + naga 校验单测 | 演示 mod 就是翻译器的"回归测试集",扩展语法时应同步扩展它 |
| `gpu/` | 桩库(**glam 再导出**:Vec2/3/4、Mat2/3/4、IVec*/UVec*;array<T>、纹理/采样器 handle 家族、`ConstDefault` trait + 原生/glam impl、数学自由函数;**构造宏** vec2f!/vec3f!/vec4f!/mat2x2f!/mat3x3f!/mat4x4f!)。代码按主题拆子模块:`vec2/3/4.rs`、`mat2x2/3x3/4x4.rs`、`math.rs`、`texture.rs`、`arrays.rs`、`const_default.rs`,`lib.rs` 只做再导出 | **规则:只实现"必须的功能"**——向量/矩阵直接来自 glam(真类型、真运算,重放副本用真语义检查);array/handle/数学桩保持 no-op(`unimplemented!()`);`gpu` 依赖 glam,给 glam 类型补 `ConstDefault` impl 合法(trait 本地)。构造宏是 `#[macro_export]`,挂在 crate 根,`use gpu::*;` 就能带进 shader |
| `gpu-macro/` | proc-macro:`#[shader]` 翻译器、透传属性宏、`ConstDefault` derive | 翻译逻辑都在这;`ConstDefault` 的 **trait 在 gpu**、**derive 在 gpu-macro**,使用处两者都要引入(derive 生成的是不带路径的 `impl ConstDefault`,trait 必须在作用域里) |

## 命令

```bash
cargo run      # 打印演示 shader 翻译出的 WGSL
cargo test     # rustc 编译(重放副本类型检查)+ naga parse + Validator 完整校验
cargo check -p gpu-macro   # 只查宏 crate
```

改完翻译器必须 `cargo test` 通过(尤其改了打印逻辑后),不能只 `cargo check`。

## 不可破坏的规则(invariants)

1. **桩库只实现"必须的功能"**。array 的下标、handle、运算符语义必须真实——重放副本要过 rustc,凭空返回 `&T` 之类是过不去的;但 WGSL **数学/纹理内建函数保持 `unimplemented!()`**(CPU 上永不执行 shader)。给桩库加语义前想清楚"是不是不加就 typecheck 不过"。当前例外:**向量/矩阵 = glam**(真类型、真字段、全部运算真实,顺带让 CPU 侧共享 struct 能用);`array<T>` 是"假容器"(单元素 phantom,Index 忽略下标,仅 typecheck)。
2. **只输出 WGSL 标准里有的语法**。Rust 有但 WGSL 没有的(match、if 当表达式、引用、泛型 struct……)→ 翻译时报 `syn::Error`(带源码 span),**绝不静默跳过或伪造翻译**。
3. **单一来源**:shader 只写一遍。rustc 类型检查靠宏"重放剥掉装饰属性的原代码"完成;任何新装饰属性必须同时处理 `strip_*`(剥掉),否则重放副本编译失败。
4. **错误信息用中文**,通过 `err(node, "...")` / `syn::Error::new_spanned` 产生,span 指向出问题的源码节点。
5. **桩库 handle/函数名 = WGSL 名**(小写),靠 `#![allow(non_camel_case_types, non_snake_case)]` 豁免;**glam 类型是例外**——它们在源码里是原名(Vec4/Mat4…),由翻译器的 `glam_type_shape`/`map_wgsl_type` 译回 WGSL,不要求小写。

## 翻译器内部地图(gpu-macro/src/lib.rs)

- **装饰属性** → WGSL `@xxx`:
  - `is_decoration(attr)`:判定哪些属性是"翻译用"的(当前:group/binding/vertex/fragment/compute/builtin/location/workgroup_size/interpolate/**storage**/**align**/**size**);
  - `attr_decor(attr)` → `(名字, "@...")` 文本;非装饰属性返回 None。注意 `storage` 例外:它不走 `@` 文本,而是被 `trans_static` 单独解析成地址空间(见下)。
- **Ctx(带上下文打印器)**:
  - `Ctx.field_order`:模块内 struct 名 → 成员声明顺序(Rust 命名字面量 → WGSL 位置构造器靠它);
  - `print_expr` / `print_stmt` / `print_block` / `print_if_stmt` / `render_loop_body` / `print_for`:表达式与语句递归打印;
  - `trans_fn`:入口/普通函数(参数装饰、返回值装饰、函数体)。
- **模块级**:`trans_static`(static 上只允许 group/binding/storage,**其他装饰如 `#[align]` 会被拒绝**;值类型 → var<uniform>;**texture_2d/cube/2d_array/depth 等 handle 类型与 sampler/sampler_comparison 不加地址空间**;**`#[storage]` / `#[storage(read_write)]` → `<storage>` / `<storage, read_write>`**,可写 buffer 用 `static mut` + `unsafe`)、`trans_struct`、`trans_const`(模块级 const → WGSL const)、`trans_module`(先收集 field_order;再把 **const 整体提前输出**——WGSL 要求先声明后使用)。
- **`unsafe`**:`unsafe fn` 与 `unsafe {}` 块都是 Rust-only(写 `static mut` 的必要手段),翻译时**透明剥掉**;块当表达式用会报错。对应宏在重放 mod 上加的 allow 含 `static_mut_refs`。
- **宏入口 `shader`**:翻译(出错则直接返回编译错误)→ 剥装饰 → 重放 + `pub const WGSL`。文件底部还有一组**透传属性宏**(展开 = 原样返回),让装饰属性在 `#[shader]` 外也不报错。
- **改名表 `map_wgsl_name`**:Rust 没有函数重载 → WGSL 同名不同签名的内建在桩库拆成不同 Rust 名(texture_sample_cube/array/depth → `textureSample`、texture_sample_compare → `textureSampleCompare`、texture_load_cube → `textureLoad`、texture_dimensions_array → `textureDimensions`)。给这类函数加桩时记得同时扩表。
- **glam/构造宏映射(全 glam 迁移)**:`glam_type_shape(name)` 是唯一定义"构造目标 → (WGSL 基底, 元素类型, 维数)"的地方——同时收 **glam 类型名**(Vec2/3/4、IVec*/UVec*、Mat2/3/4)与 **构造宏名**(vec2f!/vec3f!/vec4f!/mat2x2f!/mat3x3f!/mat4x4f!);`print_type` 靠它把声明位置的类型译回 WGSL(`Vec4` → `vec4<f32>`、`UVec3` → `vec3<u32>`);`print_expr` 的 Call 分支靠它处理 `Vec4::new/splat`、`MatN::from_cols` 调用;`print_expr` 的 `Expr::Macro` 分支靠它把构造宏译成 WGSL 构造器(1 参=splat/对角显式展开、N 标量全填、矩阵列传/`dim²` 标量),参数用 `Punctuated::parse_terminated` 解析。**glam 常量(`Vec4::ZERO` 等)、方法调用与白名单外的宏译不了 → 报错**;static 初值会被整体丢弃(trans_static 不打印初值),所以那里可以放心用常量/宏。加类型/构造宏时只改这一处表 + gpu 对应宏文件。
- **重载模拟(自由函数接收元组)**:`Expr::Call` 里 `textureLoad((…))`(单个元组参数)会被摊平成 WGSL `textureLoad(…)`(见 gpu 的 `TextureLoad` trait 与三个 impl,带关联类型 `Output = Vec4`);缺参/错形会先在宏内被 naga 或 rustc 拦下。其他方法调用一律报错。注意元组里裸字面量在泛型位置可能推断不出类型,调用处用有类型的参数/变量。
- **turbofish 规则(历史遗留)**:`vec2::<f32>(..)` → `vec2<f32>(..)` 是桩 vec 时代的语法,全 glam 迁移后向量构造走 `Vec4::new(..)`,此分支仅剩"其他显式泛型调用"时触发;syn 里泛型参数在 path 的 `AngleBracketed` 里,显式可读,翻译器不需要类型推断。

## 加一种语法的标准动作(清单)

1. 确定目标 WGSL 语义;**先确认 WGSL 标准里有它**(否则按规则 2 拒绝并写清报错文案)。
2. 在 `Ctx` 对应打印函数里加分支(表达式 → `print_expr`,语句 → `print_stmt`,模块级 → `trans_module`)。
3. 如果是新属性:加进 `is_decoration`(若语义上需要翻译)与透传宏列表;新出现在 struct 成员/参数上的属性,记得在 `strip_struct`/`strip_fn` 里剥掉。
4. 扩展 `src/main.rs` 的演示 shader,让新语法进 naga 校验范围。
5. 更新 `README.md` 的"支持的语法"清单(以及"不支持的语法"里删除/补充)。
6. `cargo test` 全绿再收工。

## 踩过的坑(别再踩)

- `quote!(#b.op)` 不会取字段——`#b` 之后跟 `.op` 会被当字面量 token 输出;取运算符/字面量原文用 `xxx.to_token_stream().to_string()`。
- syn 2 API:局部初值是 `Local.init: Option<LocalInit>`(`init.expr` 才是表达式);`ItemStatic.mutability` 是 `StaticMutability` 枚举(用 `matches!`);`RangeLimits::HalfOpen/Closed` 是元组变体;`ExprBlock` 的块在 `.block.stmts`。
- `Local.pat` 是 `Pat` 不是 `Box<Pat>`(直接 `&local.pat`);`ForLoop.pat/expr` 是 `Box`(要 `&*f.pat`)。
- syn 需要 `printing` feature 才有 `ToTokens` 实现(gpu-macro 已开 `full, parsing, printing`)。
- 重放副本里的 lint(dead_code/unused_variables/non_upper_case_globals 等)靠用户源码里的 `#[allow]` 或宏在重放 mod 上加的 `#[allow(dead_code, unused_imports, static_mut_refs)]` 压掉;新增 lint 噪音时优先改 demo 源码而不是放宽宏。
- 写 storage 的 `static mut` 让 rustc 产生 `static_mut_refs` lint(宏已在重放 mod 上 allow);`static mut` 只允许出现在 `#[storage(read_write)]`,否则翻译器报错。别把变量名起成和辅助函数同名(曾踩 `storage_mode` 遮蔽函数)。
- `array<T>` 是"假容器"(内部一个 `phantom` 元素,Index/IndexMut 忽略下标,只为 typecheck 过),用于 storage buffer 成员;**数组字面量/定长数组 `array<T,N>` 还没做**,遇到先报错而不是硬编。
- WGSL 要求模块内"先声明后使用":const 已被自动提前,但 struct/static 仍按**源顺序**输出——演示 mod 里把声明放在使用它们的函数前面,别依赖 Rust 的顺序无关性。
- `ConstDefault` derive 只支持 struct:命名字段 `Self { .. }`、元组字段 `Self(..)`、单元 `Self`,三者构造写法不同(曾因一律 `Self(...)` 让命名字段 struct 报 `expected identifier, found ':'`);enum/union 给编译错误,别在宏里 `panic!`。derive 生成的 impl 引用不带路径的 `ConstDefault`,使用处必须把 trait 引进来。
- **macro_rules 固定元数 arm 记得给尾逗号留口**:调用点多行书写爱带尾逗号,模式写成 `($x:expr, $y:expr $(,)?) => …`(expr 片段可以匹配嵌套宏调用,如 `mat3x3f!(vec3f!(…), …)` 没问题);忘了 `$(,)?` 会静默落到 fallback 的 `compile_error!`,报错信息让人误以为是参数种类不对。
