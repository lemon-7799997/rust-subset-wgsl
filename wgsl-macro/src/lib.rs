//! # 实验 crate:混合解析的 `wgsl! { ... }`
//!
//! 学习目的(宏三个核心事实):
//!   1. proc-macro(函数式宏)拿到的是**不透明 token 流** —— rustc 只负责词法,
//!      语法由我们自己按需解析,能合法解析 Rust 的部分就用 syn,解析不了的
//!      就自己手写。
//!   2. 顶层"切块"是混合架构的第一步:按 token 形状(顶层 `;` 结束一个声明,
//!      顶层 `{...}` 结束一个 fn/struct)把输入切成一个个独立块。
//!   3. 每块先试 `syn::parse2::<syn::Item>`(合法 Rust 就当 Rust item 处理),
//!      失败就走自写的小解析器(WGSL 专属语法,如 `var<uniform> ...;`)。
//!
//! 这个最小试验不做翻译,只做"分类 + 摘要",证明两种解析路径能在同一个
//! 宏里共存。产物是一个编译期生成的摘要常量,方便在运行时打印观察。

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::quote;

// ============================================================================
// 1. 顶层切块
// ============================================================================

/// 判断一个块是不是"花括号体"形状(fn/struct/impl ... 以顶层 `{...}` 收尾)。
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

/// 把输入的 token 流切成顶层块:
///   - 顶层 `;` → 一个块结束(var<uniform> u_scale: f32; 这种);
///   - 顶层 `{...}` 且块以 fn/struct 等开头 → 一个块结束(fn main() { ... })。
/// 花括号内部的内容在 token 流里是"一个 Group",内部的 `;` 不会出现在顶层,
/// 所以这个切分不需要维护深度计数器。
fn split_top_level(input: TokenStream2) -> Vec<TokenStream2> {
    let trees: Vec<TokenTree> = input.into_iter().collect();
    let mut chunks: Vec<TokenStream2> = Vec::new();
    let mut cur: Vec<TokenTree> = Vec::new();

    for tt in trees {
        let ends_chunk = match &tt {
            // 顶层分号: 声明类块
            TokenTree::Punct(p) if p.as_char() == ';' => true,
            // 顶层花括号 + 当前块开头是 fn/struct...: 函数/结构体类块
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
// 2. 逐块分类:能当 Rust item 就交给 syn,否则自写解析器
// ============================================================================

fn token_text(tokens: &TokenStream2) -> String {
    tokens.to_string()
}

/// 解析 WGSL 专属的 `var<地址空间> 名字: 类型;`(手写小解析器,纯 token 匹配)。
fn describe_wgsl_var(chunk: &TokenStream2) -> String {
    let tt: Vec<TokenTree> = chunk.clone().into_iter().collect();

    // 期望形状: [var][<][addr][>][name][:][...ty...][;]
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
            (addr.to_string(), name.to_string())
        }
        _ => return format!("不认识 / 未支持的 WGSL 顶层片段: {}", token_text(chunk)),
    };

    // 类型部分: 冒号后到分号前 —— 这是"两边语法一致"的部分,
    // 可以直接用 syn 解析成 Type 来验证(展示 syn 复用)。
    let mut ty_vec: Vec<TokenTree> = tt[6..].to_vec();
    if matches!(ty_vec.last(), Some(TokenTree::Punct(p)) if p.as_char() == ';') {
        ty_vec.pop(); // 剥掉结尾分号再交给 syn
    }
    let ty_tokens: TokenStream2 = ty_vec.into_iter().collect();
    let ty_text = token_text(&ty_tokens);
    match syn::parse2::<syn::Type>(ty_tokens.clone()) {
        Ok(ty) => {
            // 证明"类型这块真的用 syn 解析过了"
            let _ = ty;
            format!(
                "WGSL 声明 var<{}> {}: {};(类型部分经 syn::Type 解析)",
                head.0, head.1, ty_text
            )
        }
        Err(_) => format!(
            "WGSL 声明 var<{}> {}: {};(类型部分 syn 解析失败,原样透传)",
            head.0, head.1, ty_text
        ),
    }
}

/// 合法的 Rust item → 用 syn 的 AST 描述它(证明真的走了 syn::Item/ItemFn)。
fn describe_rust_item(item: &syn::Item) -> String {
    match item {
        syn::Item::Fn(f) => format!(
            "合法 Rust fn `{}`(syn::ItemFn: 参数 {} 个, 语句 {} 条)",
            f.sig.ident,
            f.sig.inputs.len(),
            f.block.stmts.len()
        ),
        syn::Item::Struct(s) => format!(
            "合法 Rust struct `{}`(syn::ItemStruct: {} 个字段)",
            s.ident,
            match &s.fields {
                syn::Fields::Named(nf) => nf.named.len(),
                syn::Fields::Unnamed(uf) => uf.unnamed.len(),
                syn::Fields::Unit => 0,
            }
        ),
        other => format!("合法 Rust item(其他类型): {}", item_kind_name(other)),
    }
}

fn item_kind_name(item: &syn::Item) -> &'static str {
    match item {
        syn::Item::Const(_) => "const",
        syn::Item::Type(_) => "type",
        syn::Item::Use(_) => "use",
        syn::Item::Mod(_) => "mod",
        syn::Item::Impl(_) => "impl",
        syn::Item::Enum(_) => "enum",
        _ => "…",
    }
}

/// 一块输入 → 摘要字符串。先试 syn(合法 Rust),失败再试自写 WGSL 解析器。
fn describe_chunk(chunk: &TokenStream2) -> String {
    if let Ok(item) = syn::parse2::<syn::Item>(chunk.clone()) {
        return format!("{}  ——  {}", token_text(chunk), describe_rust_item(&item));
    }
    describe_wgsl_var(chunk)
}

// ============================================================================
// 3. 宏入口:输出"每块摘要是啥"的编译期常量,方便运行观察
// ============================================================================

#[proc_macro]
pub fn wgsl(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    let chunks = split_top_level(input);

    let summaries: Vec<TokenStream2> = chunks
        .iter()
        .map(|chunk| {
            let s = describe_chunk(chunk);
            quote!(#s) // &str → 字符串字面量 token
        })
        .collect();

    quote! {
        /// 实验产物:wgsl! 对每一块顶层片段的解析摘要
        #[allow(non_upper_case_globals)]
        pub const CHUNKS: &[&str] = &[ #(#summaries),* ];
    }
    .into()
}
