//! # gpu-macro —— WGSL 翻译宏
//!
//! `#[shader] mod xxx { ... }`(一个 mod = 一个 WGSL 模块,单源):
//!   1. 用 syn 解析整个 mod 的 token;
//!   2. 翻译器把 AST 打印成 WGSL 文本(见下方 `trans_*`/`Ctx::print_*`);
//!   3. 把剥掉装饰属性(`#[binding]` 等,它们只是"翻译用语法")的原代码
//!      重放出来 → 让 rustc 对 gpu 桩库做类型检查;
//!   4. 在 mod 里塞 `pub const WGSL: &str = <翻译产物>;`
//!
//! 原则: 只输出 WGSL 标准里存在的语法;标准没有的东西直接编译报错,不搞
//! "看起来像"的假翻译。典型禁止项: match/模式匹配、if 当表达式、引用/指针、
//! 结构体字面量的 `..base` 更新、带 label 的循环等。
//!
//! 已支持的翻译规则:
//!   - 模块级 `static` + `#[group(..)] #[binding(..)]` → `@group(..) @binding(..) var<uniform>`
//!   - 模块内 `struct Name { ... }`:
//!       * 成员上的 `#[builtin(..)]/#[location(..)]/#[interpolate(..)]` → `@...`(成员装饰)
//!       * Rust 命名字面量 `Name { a: x, b: y }` → WGSL 位置构造器 `Name(x, y)`
//!         (按 struct 声明顺序,成员名和个数由 rustc 保证一致)
//!       * 用户 struct 不允许泛型(WGSL 没有)
//!   - fn 级 `#[vertex]/#[fragment]/#[compute]/#[workgroup_size]` → `@...`
//!   - fn 级 `#[builtin(..)]/#[location(..)]` → 挪到返回类型前(`-> @builtin(position) vec4<f32>`)
//!   - 参数属性 → 原样变成 `@...`
//!   - `let mut x = e;` → `var x = e;`;`let x = e;` → `let x = e;`
//!   - `e as f32` → `f32(e)`;`vec2::<f32>(..)` → `vec2<f32>(..)`(turbofish 的 `::` 被吃掉)
//!   - 语句: if/else if/else、loop、while、`for k in 0..N`(重写成 WGSL for 头)、
//!     break/continue、return
//!   - 超出子集的语法 → 带源码 span 的编译错误
//!
//! 未实现(下一刀): 模块级常量/array/纹理/采样器、storage buffer、swizzle、
//! `0..N` 之外的迭代、struct 布局属性(@size/@align)等。

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{
    parse_macro_input, Attribute, Expr, FnArg, GenericArgument, Item, ItemFn, ItemMod, ItemStatic,
    ItemStruct, Meta, Pat, ReturnType, Stmt, Type,
};

// ============================================================================
// 装饰属性
// ============================================================================

/// "翻译用装饰属性":在 WGSL 里对应 `@xxx`。剥掉后原代码才是合法 Rust。
fn is_decoration(attr: &Attribute) -> bool {
    attr.path().segments.last().is_some_and(|s| {
        matches!(
            s.ident.to_string().as_str(),
            "group"
                | "binding"
                | "vertex"
                | "fragment"
                | "compute"
                | "builtin"
                | "location"
                | "workgroup_size"
                | "interpolate"
                | "storage"
        )
    })
}

/// 装饰属性 → (属性名, 目标 WGSL 文本),例如 ("binding", "@binding(0)")。
/// 非装饰属性(allow/doc/...)返回 None —— 它们只是 Rust 侧的东西。
fn attr_decor(attr: &Attribute) -> Option<(String, String)> {
    if !is_decoration(attr) {
        return None;
    }
    let path = match &attr.meta {
        Meta::Path(p) => p,
        Meta::List(l) => &l.path,
        Meta::NameValue(n) => &n.path,
    };
    let name = path.segments.last()?.ident.to_string();
    let text = match &attr.meta {
        Meta::Path(_) => format!("@{name}"),
        Meta::List(l) => format!("@{name}({})", l.tokens.to_string()),
        Meta::NameValue(_) => return None, // @name = value 形式 WGSL 没有
    };
    Some((name, text))
}

// ============================================================================
// 翻译器: syn AST → WGSL 文本
// ============================================================================

fn err<T: ToTokens>(node: &T, what: impl std::fmt::Display) -> syn::Error {
    syn::Error::new_spanned(node, format!("翻译器暂不支持: {what}"))
}

fn path_last(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

fn print_generic_arg(arg: &GenericArgument) -> Result<String, syn::Error> {
    match arg {
        GenericArgument::Type(t) => print_type(t),
        // array<f32, 3> 之类的常量参数(以后用)
        GenericArgument::Const(c) => Ok(c.to_token_stream().to_string()),
        other => Err(err(other, "这种泛型参数")),
    }
}

fn print_type(ty: &Type) -> Result<String, syn::Error> {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => {
            let seg = match tp.path.segments.last() {
                Some(s) => s,
                None => return Err(err(ty, "空路径类型")),
            };
            let name = seg.ident.to_string();
            match &seg.arguments {
                syn::PathArguments::None => Ok(name),
                syn::PathArguments::AngleBracketed(ab) => {
                    let inner = ab
                        .args
                        .iter()
                        .map(print_generic_arg)
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");
                    Ok(format!("{name}<{inner}>"))
                }
                syn::PathArguments::Parenthesized(_) => Err(err(ty, "括号类型参数")),
            }
        }
        Type::Paren(p) => print_type(&p.elem), // 剥掉一层括号
        other => Err(err(other, "这种类型写法")),
    }
}

/// 值路径:子集只允许单个标识符(可能有 turbofish 泛型)。
fn print_value_path(path: &syn::Path) -> Result<String, syn::Error> {
    if path.segments.len() != 1 {
        return Err(err(path, "多段路径(`::` 只在 turbofish 里出现)"));
    }
    let seg = &path.segments[0];
    let name = seg.ident.to_string();
    match &seg.arguments {
        syn::PathArguments::None => Ok(name),
        syn::PathArguments::AngleBracketed(ab) => {
            let inner = ab
                .args
                .iter()
                .map(print_generic_arg)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(format!("{name}<{inner}>"))
        }
        syn::PathArguments::Parenthesized(_) => Err(err(path, "括号类型参数")),
    }
}

/// 翻译上下文:`field_order` 记录本 shader mod 里所有 struct 的成员声明顺序,
/// 用于把 Rust 命名结构体字面量还原成 WGSL 的位置构造器。
struct Ctx<'a> {
    field_order: &'a HashMap<String, Vec<String>>,
}

impl Ctx<'_> {
    fn print_expr(&self, e: &Expr) -> Result<String, syn::Error> {
        match e {
            Expr::Lit(lit) => Ok(lit.lit.to_token_stream().to_string()),
            Expr::Path(p) => print_value_path(&p.path),
            Expr::Call(call) => {
                let func = match &*call.func {
                    Expr::Path(p) => p,
                    other => return Err(err(other, "调用目标必须是函数名/类型构造器")),
                };
                let args = call
                    .args
                    .iter()
                    .map(|a| self.print_expr(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let name = path_last(&func.path);
                // 带 turbofish 泛型 → 类型构造器: vec2::<f32>(..) → vec2<f32>(..)
                if let Some(seg) = func.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        let tys = ab
                            .args
                            .iter()
                            .map(print_generic_arg)
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ");
                        return Ok(format!("{name}<{tys}>({args})"));
                    }
                }
                Ok(format!("{name}({args})"))
            }
            Expr::Binary(b) => {
                let op = b.op.to_token_stream().to_string();
                Ok(format!(
                    "({} {op} {})",
                    self.print_expr(&b.left)?,
                    self.print_expr(&b.right)?
                ))
            }
            Expr::Unary(u) => {
                let op = u.op.to_token_stream().to_string();
                if op == "-" || op == "!" {
                    Ok(format!("{op}{}", self.print_expr(&u.expr)?))
                } else {
                    Err(err(u, "一元运算符(指针相关)"))
                }
            }
            Expr::Field(f) => match &f.member {
                syn::Member::Named(ident) => Ok(format!("{}.{}", self.print_expr(&f.base)?, ident)),
                syn::Member::Unnamed(_) => Err(err(f, "元组字段 .0/.1")),
            },
            Expr::Index(i) => Ok(format!(
                "{}[{}]",
                self.print_expr(&i.expr)?,
                self.print_expr(&i.index)?
            )),
            Expr::Cast(c) => {
                // vid as f32 → f32(vid)(WGSL 转换构造器)
                Ok(format!(
                    "{}({})",
                    print_type(&c.ty)?,
                    self.print_expr(&c.expr)?
                ))
            }
            Expr::Paren(p) => Ok(format!("({})", self.print_expr(&p.expr)?)),
            Expr::Assign(a) => Ok(format!(
                "{} = {}",
                self.print_expr(&a.left)?,
                self.print_expr(&a.right)?
            )),
            Expr::Return(r) => {
                // 单独出现时一般会走 Stmt 分支,这里兜底
                match r.expr.as_deref() {
                    Some(x) => Ok(format!("return {};", self.print_expr(x)?)),
                    None => Ok("return;".into()),
                }
            }
            Expr::Struct(s) => {
                // Rust 命名字面量 → WGSL 位置构造器(顺序 = struct 声明顺序)
                if s.rest.is_some() {
                    return Err(err(s, "`..base` 更新语法"));
                }
                let name = path_last(&s.path);
                let has_generics = s
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| !matches!(seg.arguments, syn::PathArguments::None));
                if has_generics {
                    return Err(syn::Error::new_spanned(
                        s,
                        "gpu 的向量/矩阵等请用 vec2::<f32>(...) 构造器, 不要写结构体字面量",
                    ));
                }
                let order = self.field_order.get(&name).ok_or_else(|| {
                    syn::Error::new_spanned(
                        &s.path,
                        format!("struct `{name}` 必须在本 shader mod 里定义才能这样构造"),
                    )
                })?;
                if s.fields.len() != order.len() {
                    return Err(err(s, "struct 字面量字段数与定义不一致"));
                }
                let mut by_name: HashMap<String, &Expr> = HashMap::new();
                for fv in &s.fields {
                    if let syn::Member::Named(id) = &fv.member {
                        by_name.insert(id.to_string(), &fv.expr);
                    }
                }
                let mut args = Vec::new();
                for member in order {
                    match by_name.get(member) {
                        Some(e) => args.push(self.print_expr(e)?),
                        None => return Err(err(s, format!("struct 字面量缺少字段 `{member}`"))),
                    }
                }
                Ok(format!("{name}({})", args.join(", ")))
            }
            Expr::If(_) | Expr::While(_) | Expr::Loop(_) | Expr::ForLoop(_) | Expr::Block(_) => {
                Err(syn::Error::new_spanned(
                    e,
                    "控制流在 WGSL 里只能是语句,不能当表达式用",
                ))
            }
            Expr::Match(m) => Err(syn::Error::new_spanned(
                m,
                "禁止: WGSL 标准没有 match/模式匹配(本翻译只做 WGSL 存在的语法), 请改写 if/else",
            )),
            Expr::Unsafe(u) => Err(syn::Error::new_spanned(
                u,
                "unsafe 块只能以语句形式出现(如包住对 static mut 的写), 不能当表达式用",
            )),
            Expr::Break(_) | Expr::Continue(_) => {
                Err(syn::Error::new_spanned(e, "break/continue 只能作为语句"))
            }
            other => Err(err(other, "这种表达式")),
        }
    }

    fn print_stmt(&self, stmt: &Stmt, pad: &str) -> Result<String, syn::Error> {
        match stmt {
            Stmt::Local(local) => {
                let Pat::Ident(pi) = &local.pat else {
                    return Err(err(&local.pat, "let 解构/复杂模式"));
                };
                if pi.by_ref.is_some() {
                    return Err(err(pi, "let ref"));
                }
                let kw = if pi.mutability.is_some() {
                    "var"
                } else {
                    "let"
                };
                // WGSL 局部必须有初值
                let init = match local.init.as_ref() {
                    Some(li) => &li.expr,
                    None => return Err(err(local, "没有初值的局部变量(WGSL 不允许)")),
                };
                let value = self.print_expr(init)?;
                Ok(format!("{pad}{kw} {} = {value};", pi.ident))
            }
            Stmt::Item(item) => Err(err(item, "函数体内的 item 定义")),
            Stmt::Expr(e, _) => match e {
                Expr::Return(r) => {
                    let inner = match r.expr.as_deref() {
                        Some(x) => self.print_expr(x)?,
                        None => String::new(),
                    };
                    Ok(if inner.is_empty() {
                        format!("{pad}return;")
                    } else {
                        format!("{pad}return {inner};")
                    })
                }
                Expr::If(i) => self.print_if_stmt(i, pad),
                Expr::While(w) => {
                    if w.label.is_some() {
                        return Err(err(w, "带 label 的循环"));
                    }
                    let header = format!("{pad}while {} {{", self.print_expr(&w.cond)?);
                    self.render_loop_body(&header, &w.body, pad)
                }
                Expr::Loop(l) => {
                    if l.label.is_some() {
                        return Err(err(l, "带 label 的循环"));
                    }
                    self.render_loop_body(&format!("{pad}loop {{"), &l.body, pad)
                }
                Expr::ForLoop(f) => self.print_for(f, pad),
                Expr::Continue(_) => Ok(format!("{pad}continue;")),
                Expr::Break(b) => {
                    if b.expr.is_some() {
                        return Err(err(b, "`break 值`(WGSL 的 break 不带值)"));
                    }
                    Ok(format!("{pad}break;"))
                }
                Expr::Block(b) => Err(err(b, "裸块语句")),
                // Rust-only 包装(如写 static mut 的 unsafe),翻译时透明展开
                Expr::Unsafe(u) => self.print_block(&u.block.stmts, pad),
                _ => Ok(format!("{pad}{};", self.print_expr(e)?)),
            },
            Stmt::Macro(m) => Err(err(m, "宏语句")),
        }
    }

    /// 打印一段语句列表,每条语句自带 `pad` 前缀。
    fn print_block(&self, stmts: &[Stmt], pad: &str) -> Result<String, syn::Error> {
        let mut parts = Vec::new();
        for stmt in stmts {
            parts.push(self.print_stmt(stmt, pad)?);
        }
        Ok(parts.join("\n"))
    }

    /// 通用循环体: `header` 已经带 pad,函数负责缩进内部并收尾 `}`。
    fn render_loop_body(
        &self,
        header: &str,
        body: &syn::Block,
        pad: &str,
    ) -> Result<String, syn::Error> {
        let inner = format!("{pad}    ");
        let body = self.print_block(&body.stmts, &inner)?;
        Ok(if body.is_empty() {
            format!("{header}\n{pad}}}")
        } else {
            format!("{header}\n{body}\n{pad}}}")
        })
    }

    /// if / else if / else 链。
    fn print_if_stmt(&self, i: &syn::ExprIf, pad: &str) -> Result<String, syn::Error> {
        let inner = format!("{pad}    ");
        let mut lines: Vec<String> = Vec::new();
        let mut cur: Option<&syn::ExprIf> = Some(i);
        let mut head = format!("{pad}if ");
        while let Some(node) = cur {
            let cond = self.print_expr(&node.cond)?;
            lines.push(format!("{head}{cond} {{"));
            let body = self.print_block(&node.then_branch.stmts, &inner)?;
            if !body.is_empty() {
                lines.extend(body.lines().map(str::to_string));
            }
            match &node.else_branch {
                None => {
                    lines.push(format!("{pad}}}"));
                    break;
                }
                Some((_, e)) => match &**e {
                    Expr::If(ni) => {
                        cur = Some(ni);
                        head = format!("{pad}}} else if ");
                    }
                    Expr::Block(b) => {
                        lines.push(format!("{pad}}} else {{"));
                        let body2 = self.print_block(&b.block.stmts, &inner)?;
                        if !body2.is_empty() {
                            lines.extend(body2.lines().map(str::to_string));
                        }
                        lines.push(format!("{pad}}}"));
                        break;
                    }
                    other => return Err(err(other, "else 分支必须是块或 else if")),
                },
            }
        }
        Ok(lines.join("\n"))
    }

    /// Rust `for k in 0..n` → WGSL `for (var k = 0; k < n; k = k + 1)`。
    fn print_for(&self, f: &syn::ExprForLoop, pad: &str) -> Result<String, syn::Error> {
        if f.label.is_some() {
            return Err(err(f, "带 label 的循环"));
        }
        let Pat::Ident(pi) = &*f.pat else {
            return Err(err(&f.pat, "for 的迭代变量只支持普通标识符"));
        };
        let Expr::Range(r) = &*f.expr else {
            return Err(err(&f.expr, "for 只支持 `0..N` / `0..=N` 区间形式"));
        };
        let start = match r.start.as_deref() {
            Some(s) => self.print_expr(s)?,
            None => return Err(err(r, "for 区间缺起点")),
        };
        let end = match r.end.as_deref() {
            Some(e) => self.print_expr(e)?,
            None => return Err(err(r, "for 区间缺终点(开区间不支持)")),
        };
        let cmp = match r.limits {
            syn::RangeLimits::HalfOpen(_) => "<",
            syn::RangeLimits::Closed(_) => "<=",
        };
        let name = &pi.ident;
        let header =
            format!("{pad}for (var {name} = {start}; {name} {cmp} {end}; {name} = {name} + 1) {{");
        self.render_loop_body(&header, &f.body, pad)
    }

    fn trans_fn(&self, f: &ItemFn) -> Result<String, syn::Error> {
        // fn 级装饰分类: stage 属性 / 返回值装饰 / 非法位置
        let mut stage = Vec::new(); // @vertex 等,放在 fn 前面
        let mut ret_decor = Vec::new(); // @builtin(position) 等,挪到 -> 后面
        for attr in &f.attrs {
            if let Some((name, text)) = attr_decor(attr) {
                match name.as_str() {
                    "vertex" | "fragment" | "compute" | "workgroup_size" => stage.push(text),
                    "builtin" | "location" => ret_decor.push(text),
                    "group" | "binding" => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            "#[group]/#[binding] 只能用在模块级 static 上",
                        ))
                    }
                    _ => return Err(err(attr, "fn 上的这种装饰属性")),
                }
            }
        }

        // 参数: 装饰属性 + 名字 + 类型
        let mut params = Vec::new();
        for input in &f.sig.inputs {
            match input {
                FnArg::Typed(pt) => {
                    let decor: Vec<String> = pt
                        .attrs
                        .iter()
                        .filter_map(attr_decor)
                        .map(|(_, t)| t)
                        .collect();
                    let Pat::Ident(pi) = &*pt.pat else {
                        return Err(err(&pt.pat, "参数模式只支持普通标识符"));
                    };
                    let ty = print_type(&pt.ty)?;
                    let head = decor.join(" ");
                    let item = if head.is_empty() {
                        format!("{}: {ty}", pi.ident)
                    } else {
                        format!("{head} {}: {ty}", pi.ident)
                    };
                    params.push(item);
                }
                FnArg::Receiver(r) => return Err(err(r, "self/引用参数")),
            }
        }

        // 返回值装饰必须有返回类型;没有 → 报错而不是静默丢掉装饰
        if !ret_decor.is_empty() && matches!(f.sig.output, ReturnType::Default) {
            return Err(syn::Error::new_spanned(
                &f.sig.output,
                "fn 上的 #[builtin(..)]/#[location(..)] 是\"返回值装饰\",要求函数声明返回类型; \
                 没有返回类型时这些装饰没有可挂的地方(WGSL 入口要么返回位置要么返回 struct 成员)",
            ));
        }
        // 返回类型 + 返回值装饰
        let ret = match &f.sig.output {
            ReturnType::Default => String::new(),
            ReturnType::Type(_, ty) => {
                let ty = print_type(ty)?;
                if ret_decor.is_empty() {
                    format!(" -> {ty}")
                } else {
                    format!(" -> {} {ty}", ret_decor.join(" "))
                }
            }
        };

        // 函数体语句(print_block 自带一级缩进)
        let body = self.print_block(&f.block.stmts, "    ")?;

        let pre = if stage.is_empty() {
            String::new()
        } else {
            format!("{}\n", stage.join(" "))
        };
        let sig = format!("{pre}fn {}({}){ret}", f.sig.ident, params.join(", "));
        Ok(if body.is_empty() {
            format!("{sig} {{}}")
        } else {
            format!("{sig} {{\n{body}\n}}")
        })
    }
}

/// `#[storage]` / `#[storage(read)]` / `#[storage(read_write)]` → storage 访问模式。
fn storage_mode(attr: &Attribute) -> Result<String, syn::Error> {
    match &attr.meta {
        Meta::Path(_) => Ok("read".into()), // 省略模式 = read(规范默认)
        Meta::List(l) => {
            let mode = l.tokens.to_string();
            if matches!(mode.as_str(), "read" | "read_write") {
                Ok(mode)
            } else {
                Err(syn::Error::new_spanned(
                    attr,
                    "storage 只支持 #[storage] / #[storage(read)] / #[storage(read_write)]",
                ))
            }
        }
        Meta::NameValue(_) => Err(syn::Error::new_spanned(
            attr,
            "storage 不能用 name=value 形式",
        )),
    }
}

fn trans_static(s: &ItemStatic) -> Result<String, syn::Error> {
    let mut decors = Vec::new();
    let (mut has_group, mut has_binding) = (false, false);
    // storage_mode: None=没写 storage;Some("read"|"read_write")=#[storage(...)]
    let mut storage_attr: Option<String> = None;
    for attr in &s.attrs {
        if let Some((name, text)) = attr_decor(attr) {
            match name.as_str() {
                "group" => has_group = true,
                "binding" => has_binding = true,
                "storage" => {
                    storage_attr = Some(storage_mode(attr)?);
                    continue; // 不是 @ 装饰,是地址空间
                }
                _ => {}
            }
            decors.push(text);
        }
    }
    if !(has_group && has_binding) {
        return Err(syn::Error::new_spanned(
            s,
            "模块级 static 需要 #[group(..)] + #[binding(..)](uniform / storage / texture / sampler 声明)",
        ));
    }
    // 可写 storage 在 Rust 侧用 `static mut` 表达(Rust-only 手段,
    // 写入要包 unsafe 块,翻译时 unsafe 被透明剥掉)。
    let is_mut = matches!(s.mutability, syn::StaticMutability::Mut(_));
    match (&storage_attr, is_mut) {
        (Some(mode), true) if mode == "read_write" => {}
        (Some(_), true) => {
            return Err(syn::Error::new_spanned(
                s,
                "写 storage 需要 #[storage(read_write)] + `static mut`",
            ))
        }
        (None, true) => return Err(err(s, "static mut 只能用于 #[storage(read_write)] buffer")),
        _ => {}
    }

    let ty_text = print_type(&s.ty)?;
    // handle 类型(texture/sampler)在 WGSL 里没有地址空间:
    //   @group(0) @binding(1) var tex: texture_2d<f32>;
    let is_handle =
        match s.ty.as_ref() {
            Type::Path(tp) if tp.qself.is_none() => tp.path.segments.last().is_some_and(|seg| {
                matches!(seg.ident.to_string().as_str(), "texture_2d" | "sampler")
            }),
            _ => false,
        };
    // 地址空间三选一: handle(无)/ storage(...)/ uniform
    let addr = match (&storage_attr, is_handle) {
        (Some(_), true) => {
            return Err(syn::Error::new_spanned(
                s,
                "handle 类型(texture/sampler)不能是 #[storage] buffer",
            ))
        }
        (Some(mode), false) => {
            if mode == "read_write" {
                "<storage, read_write>"
            } else {
                "<storage>"
            }
        }
        (None, true) => "",
        (None, false) => "<uniform>",
    };
    // 初值 `= ...` 丢弃:WGSL 的 var 声明没有初值
    Ok(format!(
        "{} var{addr} {}: {ty_text};",
        decors.join(" "),
        s.ident
    ))
}

fn trans_struct(s: &ItemStruct) -> Result<String, syn::Error> {
    if !s.generics.params.is_empty() {
        return Err(err(&s.generics, "用户 struct 不支持泛型(WGSL 没有)"));
    }
    let syn::Fields::Named(nf) = &s.fields else {
        return Err(err(&s.fields, "只支持普通命名 struct `Name { ... }`"));
    };
    if nf.named.is_empty() {
        return Err(err(s, "空 struct(WGSL 要求至少一个成员)"));
    }
    let mut members = Vec::new();
    for field in &nf.named {
        let decor: Vec<String> = field
            .attrs
            .iter()
            .filter_map(attr_decor)
            .map(|(_, t)| t)
            .collect();
        let name = match &field.ident {
            Some(id) => id,
            None => return Err(err(field, "tuple struct 成员")),
        };
        let ty = print_type(&field.ty)?;
        let head = decor.join(" ");
        members.push(if head.is_empty() {
            format!("    {name}: {ty},")
        } else {
            format!("    {head} {name}: {ty},")
        });
    }
    Ok(format!("struct {} {{\n{}\n}}", s.ident, members.join("\n")))
}

fn trans_module(module: &ItemMod) -> Result<String, syn::Error> {
    let Some((_, items)) = &module.content else {
        return Err(err(module, "`#[shader] mod name;`(没有花括号的 mod)"));
    };

    // 第一遍: 收集 struct 成员声明顺序(字面量按名字,WGSL 构造器按位置)
    let mut field_order: HashMap<String, Vec<String>> = HashMap::new();
    for item in items {
        if let Item::Struct(s) = item {
            if let syn::Fields::Named(nf) = &s.fields {
                field_order.insert(
                    s.ident.to_string(),
                    nf.named
                        .iter()
                        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
                        .collect(),
                );
            }
        }
    }

    let ctx = Ctx {
        field_order: &field_order,
    };
    let mut out = Vec::new();
    for item in items {
        let block = match item {
            Item::Use(_) => None, // Rust-only 的东西,不进 WGSL
            Item::Static(s) => Some(trans_static(s)?),
            Item::Struct(s) => Some(trans_struct(s)?),
            Item::Fn(f) => Some(ctx.trans_fn(f)?),
            other => return Err(err(other, "这种模块级 item")),
        };
        if let Some(b) = block {
            out.push(b);
        }
    }
    Ok(out.join("\n\n"))
}

// ============================================================================
// 宏入口
// ============================================================================

fn strip_attrs(attrs: &mut Vec<Attribute>) {
    attrs.retain(|a| !is_decoration(a));
}

fn strip_fn(mut f: ItemFn) -> ItemFn {
    strip_attrs(&mut f.attrs);
    for input in &mut f.sig.inputs {
        if let FnArg::Typed(pat) = input {
            strip_attrs(&mut pat.attrs);
        }
    }
    f
}

fn strip_static(mut s: ItemStatic) -> ItemStatic {
    strip_attrs(&mut s.attrs);
    s
}

fn strip_struct(mut s: ItemStruct) -> ItemStruct {
    strip_attrs(&mut s.attrs);
    // 成员上的装饰属性也要剥(如 #[builtin(position)] pos: vec4<f32>)
    if let syn::Fields::Named(nf) = &mut s.fields {
        for field in &mut nf.named {
            strip_attrs(&mut field.attrs);
        }
    }
    s
}

/// 目前支持: fn / static / struct / const / type / use / mod(不递归)。
fn strip_item(item: Item) -> Item {
    match item {
        Item::Fn(f) => Item::Fn(strip_fn(f)),
        Item::Static(s) => Item::Static(strip_static(s)),
        Item::Struct(s) => Item::Struct(strip_struct(s)),
        Item::Const(mut c) => {
            strip_attrs(&mut c.attrs);
            Item::Const(c)
        }
        Item::Type(mut t) => {
            strip_attrs(&mut t.attrs);
            Item::Type(t)
        }
        Item::Use(mut u) => {
            strip_attrs(&mut u.attrs);
            Item::Use(u)
        }
        other => other,
    }
}

/// `#[shader] mod xxx { ... }` —— 一个 mod = 一个 WGSL 模块(单源)。
#[proc_macro_attribute]
pub fn shader(_args: TokenStream, item: TokenStream) -> TokenStream {
    let module = syn::parse_macro_input!(item as ItemMod);

    // 1) 翻译成 WGSL(用带装饰的原始代码)
    let wgsl_text = match trans_module(&module) {
        Ok(t) => t,
        Err(e) => return e.to_compile_error().into(),
    };

    // 2) 剥干净后重放,让 rustc 对桩库做类型检查
    let Some((_, items)) = module.content else {
        return syn::Error::new_spanned(&module, "`#[shader]` 需要一个带花括号的 mod")
            .to_compile_error()
            .into();
    };
    let stripped: Vec<Item> = items.into_iter().map(strip_item).collect();

    let ident = &module.ident;
    let vis = &module.vis;
    let expanded = quote! {
        #[allow(dead_code, unused_imports, static_mut_refs)]
        #vis mod #ident {
            #(#stripped)*
            /// 翻译产物(WGSL)
            pub const WGSL: &str = #wgsl_text;
        }
    };
    expanded.into()
}

// ---------------------------------------------------------------------------
// 透传属性宏: 让 #[binding(0)] 这类"装饰属性"即使出现在 #[shader] 之外的
// 普通代码里也是合法属性(展开成原样,什么都不做)。在 #[shader] 内部它们
// 会被剥掉,永远不会走到这里。
// ---------------------------------------------------------------------------

macro_rules! passthrough {
    ($($name:ident),+ $(,)?) => {
        $(
            #[proc_macro_attribute]
            pub fn $name(_args: TokenStream, item: TokenStream) -> TokenStream {
                item
            }
        )+
    };
}

passthrough!(
    group,
    binding,
    vertex,
    fragment,
    compute,
    workgroup_size,
    builtin,
    location,
    interpolate,
    storage
);


#[proc_macro_derive(ConstDefault)]
pub fn derive_const_default(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let struct_name = &ast.ident;

    // 只支持 struct;enum/union 给编译错误(别在宏里 panic)
    let data = match &ast.data {
        syn::Data::Struct(s) => s,
        syn::Data::Enum(e) => {
            return syn::Error::new_spanned(&e.enum_token, "ConstDefault derive 只支持 struct")
                .to_compile_error()
                .into()
        }
        syn::Data::Union(u) => {
            return syn::Error::new_spanned(&u.union_token, "ConstDefault derive 只支持 struct")
                .to_compile_error()
                .into()
        }
    };

    // 泛型参数并添加约束(类型参数统一补 ConstDefault bound)
    let mut generics = ast.generics.clone();
    for param in &mut generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote!(ConstDefault));
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // 按字段形状生成正确的 Self 构造(三种写法不一样,不能都用 Self(...)):
    //   命名字段  ->  Self { x: ..., y: ... }
    //   元组字段  ->  Self(..., ...)
    //   单元结构体 ->  Self
    let default_expr = match &data.fields {
        syn::Fields::Named(fields) => {
            let inits = fields.named.iter().map(|f| {
                let name = f.ident.as_ref().expect("named 字段必然有 ident");
                quote! { #name: ConstDefault::DEFAULT }
            });
            quote! { Self { #(#inits),* } }
        }
        syn::Fields::Unnamed(fields) => {
            let inits = fields.unnamed.iter().map(|_| quote! { ConstDefault::DEFAULT });
            quote! { Self( #(#inits),* ) }
        }
        syn::Fields::Unit => quote! { Self },
    };

    let expanded = quote! {
        impl #impl_generics ConstDefault for #struct_name #ty_generics #where_clause {
            const DEFAULT: Self = #default_expr;
        }
    };

    expanded.into()
}