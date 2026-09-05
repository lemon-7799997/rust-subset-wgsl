//! 实验 2:`wgsl!` 发射的是**合法 Rust**——宏参数里的名字真的可解析:
//!
//!   - `var<uniform> u_scale: f32;`(不是合法 Rust)→ 自写解析后改写成
//!     `static u_scale: f32 = 0.0;`(名字 token 复用源文件那个,span 保留);
//!   - `fn use_it() { ... u_scale ... }`(合法 Rust)→ syn 验证后**原样重放**,
//!     里面的 `u_scale` 真的解析到上面那个 static(编译能过 = 类型检查成立;
//!     编辑器跳转/引用靠 span 回贴,可在此文件里试试 hover / go-to-def)。
//!
//! 跑法: cargo run --bin wgsl_demo

use wgsl_macro::wgsl;

wgsl! {
    var<uniform> u_scale: f32;

    fn use_it() -> f32 {
        let a = u_scale * 2.0;
        return a;
    }
}

fn main() {
    // use_it 由宏原样重放而来,里面引用的 u_scale 来自宏改写的 static
    println!("use_it() = {}", use_it());
    for c in CHUNKS {
        println!("{c}");
    }
}
