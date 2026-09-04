# agents.md

给在本仓库工作的 AI agent(以及人类协作者)的说明。先读 README.md 了解项目定位;这份文件讲**改代码时必须守住的规则**和**加功能的标准动作**。

## 项目一句话

`#[shader]` 属性宏把"合法 Rust 受限子集"翻译成"合法 WGSL";`gpu` 是 no-op 桩库,只让 rustc 对 shader 代码做类型检查;naga 单测保证产物合法。

## 仓库布局

| 路径 | 内容 | 改它之前要知道 |
|---|---|---|
| `src/main.rs` | 演示 shader + naga 校验单测 | 演示 mod 就是翻译器的"回归测试集",扩展语法时应同步扩展它 |
| `gpu/` | 桩库(vecN<T>、数学函数、运算符) | **约定:永不实现功能**,函数体保持 `unimplemented!()` |
| `gpu-macro/` | proc-macro:`#[shader]` 翻译器、透传属性宏 | 翻译逻辑都在这 |

## 命令

```bash
cargo run      # 打印演示 shader 翻译出的 WGSL
cargo test     # rustc 编译(重放副本类型检查)+ naga parse + Validator 完整校验
cargo check -p gpu-macro   # 只查宏 crate
```

改完翻译器必须 `cargo test` 通过(尤其改了打印逻辑后),不能只 `cargo check`。

## 不可破坏的规则(invariants)

1. **桩库 `gpu` 保持 no-op**。不要给数学函数/运算符写真实实现;除非用户明确要求"支持 CPU 仿真"。
2. **只输出 WGSL 标准里有的语法**。Rust 有但 WGSL 没有的(match、if 当表达式、引用、泛型 struct……)→ 翻译时报 `syn::Error`(带源码 span),**绝不静默跳过或伪造翻译**。
3. **单一来源**:shader 只写一遍。rustc 类型检查靠宏"重放剥掉装饰属性的原代码"完成;任何新装饰属性必须同时处理 `strip_*`(剥掉),否则重放副本编译失败。
4. **错误信息用中文**,通过 `err(node, "...")` / `syn::Error::new_spanned` 产生,span 指向出问题的源码节点。
5. **桩库类型/函数名 = WGSL 名**(小写),靠 `#![allow(non_camel_case_types, non_snake_case)]` 豁免。

## 翻译器内部地图(gpu-macro/src/lib.rs)

- **装饰属性** → WGSL `@xxx`:
  - `is_decoration(attr)`:判定哪些属性是"翻译用"的(当前:group/binding/vertex/fragment/compute/builtin/location/workgroup_size/interpolate);
  - `attr_decor(attr)` → `(名字, "@...")` 文本;非装饰属性返回 None。
- **Ctx(带上下文打印器)**:
  - `Ctx.field_order`:模块内 struct 名 → 成员声明顺序(Rust 命名字面量 → WGSL 位置构造器靠它);
  - `print_expr` / `print_stmt` / `print_block` / `print_if_stmt` / `render_loop_body` / `print_for`:表达式与语句递归打印;
  - `trans_fn`:入口/普通函数(参数装饰、返回值装饰、函数体)。
- **模块级**:`trans_static`(static → var<uniform>;**texture_2d/sampler 等 handle 类型不加地址空间**)、`trans_struct`、`trans_module`(先收集 field_order,再逐 item 翻译)。
- **宏入口 `shader`**:翻译(出错则直接返回编译错误)→ 剥装饰 → 重放 + `pub const WGSL`。文件底部还有一组**透传属性宏**(展开 = 原样返回),让装饰属性在 `#[shader]` 外也不报错。
- **turbofish 规则**:`vec2::<f32>(..)` → `vec2<f32>(..)`(吃掉 `::`);syn 里泛型参数在 path 的 `AngleBracketed` 里,显式可读,翻译器不需要类型推断。

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
- 重放副本里的 lint(dead_code/unused_variables/non_upper_case_globals 等)靠用户源码里的 `#[allow]` 或宏在重放 mod 上加的 `#[allow(dead_code, unused_imports)]` 压掉;新增 lint 噪音时优先改 demo 源码而不是放宽宏。
