//! # 实验 crate(第 2 版):`wgsl! { ... }` 混合解析 + **发射合法 Rust**
//!
//! 上一版只输出"字符串摘要",里面引用的 `u_scale` 在产物里不存在,rustc 没法
//! 检查、编辑器也没法跳转。这一版验证:
//!
//!   > 想让宏参数里的名字(u_scale)可被类型检查 / go-to-definition / find
//!   > references,唯一办法是让发射出的 Rust 里真实存在这个名字,并且发射
//!   > token 时**保留源文件 span**(原样重放原始 token 天然保留;自写解析
//!   > 改写的部分要复用原始的名字/类型 token,不要新建)。
//!
//! 处理策略(逐块双通道):
//!   - 合法 Rust 块(fn/struct...)→ `syn::parse2::<Item>` 验证后**原样重放**;
//!   - WGSL 专属块(`var<uniform> u_scale: f32;`)→ 自写解析后**改写成 Rust
//!     全局**:`static u_scale: f32 = 0.0;` —— 名字 token 用源文件里那个,
//!     span 不丢,于是 fn 里对 u_scale 的引用能解析到它。
//!
//! 学习点: token 保留 span = 语义可回贴;字符串化 = 语义丢失。

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
use quote::quote;

// ============================================================================
// 1. 顶层切块(同第 1 版:顶层 `;` 或 fn/struct 的花括号结束一个块)
// ============================================================================

fn starts_with_braced_kind(t: Option<&TokenTree>) -> bool {
    matches!(
        t,
        Some(TokenTree::Ident(id))
            if matches!(
                id.to_string().as_str(),
                "fn" | "struct" | "enum" | "impl" | "mod" | "trait" | "union"
            )
    )
}

fn split_top_level(input: TokenStream2) -> Vec<TokenStream2> {
    let trees: Vec<TokenTree> = input.into_iter().collect();
    let mut chunks: Vec<TokenStream2> = Vec::new();
    let mut cur: Vec<TokenTree> = Vec::new();

    for tt in trees {
        let ends_chunk = match &tt {
            TokenTree::Punct(p) if p.as_char() == ';' => true,
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                starts_with_braced_kind(cur.first())
            }
            _ => false,
        };
        cur.push(tt);
        if ends_chunk {
            chunks.push(cur.drain(..).collect());
        }
    }
    if !cur.is_empty() {
        chunks.push(cur.into_iter().collect());
    }
    chunks
}

// ============================================================================
// 2. WGSL 专属声明: `var<地址空间> 名字: 类型;` 的自写解析器
// ============================================================================

struct WgslVar {
    /// 地址空间(如 uniform)—— Rust 发射时用不到,留给未来翻译
    addr: String,
    /// 名字(Ident 来自源文件,span 保留)
    name: Ident,
    /// 类型 token(来自源文件)
    ty: TokenStream2,
    /// 类型的文本(用来选 Rust 默认初值)
    ty_text: String,
}

fn parse_wgsl_var(chunk: &TokenStream2) -> Result<Option<WgslVar>, String> {
    let tt: Vec<TokenTree> = chunk.clone().into_iter().collect();
    let head = match (
        tt.get(0),
        tt.get(1),
        tt.get(2),
        tt.get(3),
        tt.get(4),
        tt.get(5),
    ) {
        (
            Some(TokenTree::Ident(kw)),
            Some(TokenTree::Punct(lt)),
            Some(TokenTree::Ident(addr)),
            Some(TokenTree::Punct(gt)),
            Some(TokenTree::Ident(name)),
            Some(TokenTree::Punct(colon)),
        ) if kw == "var" && lt.as_char() == '<' && gt.as_char() == '>' && colon.as_char() == ':' =>
        {
            (addr.to_string(), name.clone())
        }
        _ => return Ok(None), // 不是 var<...> 形状,交给调用方报"不认识"
    };

    let mut ty_vec: Vec<TokenTree> = tt[6..].to_vec();
    if matches!(ty_vec.last(), Some(TokenTree::Punct(p)) if p.as_char() == ';') {
        ty_vec.pop(); // 剥掉结尾分号
    }
    let ty: TokenStream2 = ty_vec.into_iter().collect();
    Ok(Some(WgslVar {
        addr: head.0,
        name: head.1,
        ty: ty.clone(),
        ty_text: ty.to_string(),
    }))
}

/// 给 Rust 全局选一个默认初值(仅支持标量,实验够用)
fn default_init(ty_text: &str) -> Option<TokenStream2> {
    Some(match ty_text {
        "f32" => quote!(0.0),
        "i32" => quote!(0),
        "u32" => quote!(0u32),
        "bool" => quote!(false),
        _ => return None,
    })
}

// ============================================================================
// 3. 逐块发射合法 Rust
// ============================================================================

/// 一个块 → 对应的一小段合法 Rust。
///   合法 Rust 块 → 原样重放(span 全保留);
///   `var<uniform> u_scale: f32;` → `static u_scale: f32 = 0.0;`
///   (名字/类型复用源文件 token;初值是我们新建的,span 无所谓)。
fn emit_chunk(chunk: &TokenStream2) -> Result<TokenStream2, syn::Error> {
    // 通道一: 合法 Rust,直接让 syn 验证后原样放行
    if syn::parse2::<syn::Item>(chunk.clone()).is_ok() {
        return Ok(chunk.clone());
    }

    // 通道二: WGSL 专属语法,自写解析 + 改写
    let v = parse_wgsl_var(chunk)
        .map_err(|msg| syn::Error::new_spanned(chunk, msg))?
        .ok_or_else(|| {
            syn::Error::new_spanned(
                chunk,
                format!(
                    "既不是合法 Rust item,也不是支持的 WGSL 声明 `var<...> name: ty;`: {}",
                    chunk.to_string()
                ),
            )
        })?;

    let init = default_init(&v.ty_text).ok_or_else(|| {
        syn::Error::new_spanned(
            &v.ty,
            format!("实验版还不能给类型 `{}` 合成 Rust 默认初值", v.ty_text),
        )
    })?;
    let name = &v.name;
    let ty = &v.ty;

    Ok(quote! {
        // 从 `var<uniform> {}: {};` 改写而来(Rust 侧可解析的"全局")
        #[allow(non_upper_case_globals)]
        static #name: #ty = #init;
    })
}

// ============================================================================
// 4. 宏入口
// ============================================================================

#[proc_macro]
pub fn wgsl(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    match expand(input) {
        Ok(out) => out.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: TokenStream2) -> Result<TokenStream2, syn::Error> {
    let chunks = split_top_level(input);

    // 每块 → 合法 Rust(类型检查/跳转的基础)
    let mut emitted = TokenStream2::new();
    for chunk in &chunks {
        emitted.extend(emit_chunk(chunk)?);
    }

    // 附带观察用摘要(纯字符串,不参与语义)
    let summaries: Vec<TokenStream2> = chunks
        .iter()
        .map(|c| {
            let s = summarize(c);
            quote!(#s)
        })
        .collect();

    Ok(quote! {
        #emitted

        /// 观察用:每块被当成什么解析了(不影响编译/跳转)
        #[allow(non_upper_case_globals)]
        pub const CHUNKS: &[&str] = &[ #(#summaries),* ];
    })
}

/// 摘要(第 1 版保留,方便看每块走了哪条路)
fn summarize(chunk: &TokenStream2) -> String {
    if syn::parse2::<syn::Item>(chunk.clone()).is_ok() {
        return format!("[Rust item, 原样重放] {}", chunk.to_string());
    }
    match parse_wgsl_var(chunk) {
        Ok(Some(v)) => format!(
            "[WGSL var, 改写为 Rust static] var<{}> {}: {};",
            v.addr,
            v.name,
            v.ty.to_string()
        ),
        _ => format!("[未识别] {}", chunk.to_string()),
    }
}
