//! 混合解析最小试验:`wgsl! { ... }` 里混着
//!   - WGSL 专属声明(`var<uniform> u_scale: f32;` —— 不是合法 Rust);
//!   - 合法 Rust 函数(`fn main() { ... }`,注意它只是被 syn 解析观察,
//!     不会被发射出来,所以不会和本文件的真 main 冲突)。
//!
//! 跑法: cargo run --bin wgsl_demo
//! 宏内部: 顶层切块 → 每块先试 syn::Item,失败走自写 var 解析器 → 输出摘要。

use wgsl_macro::wgsl;

wgsl! {
    var<uniform> u_scale: f32;

    fn main() {
        let a = u_scale;
    }
}

fn main() {
    for c in CHUNKS {
        println!("{c}");
    }
}
