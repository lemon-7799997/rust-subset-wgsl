// ============================================================================
// 数学自由函数(no-op 桩;泛型签名,glam 向量/标量都能进)
// 名字 = WGSL 名,翻译器 1:1 透传。调用点写法与 WGSL 完全一致。
// ============================================================================

/// 一元标量/向量函数,一一对应 WGSL 内建。
macro_rules! unary_fn {
    ($($name:ident),+ $(,)?) => {
        $(pub fn $name<T>(v: T) -> T {
            let _ = v;
            unimplemented!("no-op stub: only for type checking")
        })+
    };
}

unary_fn!(
    abs,
    sign,
    fract,
    floor,
    ceil,
    round,
    trunc,
    sqrt,
    inverse_sqrt,
    exp,
    exp2,
    log,
    log2,
    sin,
    cos,
    tan,
    asin,
    acos,
    atan,
    radians,
    degrees,
);

/// 二元函数(两个同类型参数)。
macro_rules! binary_fn {
    ($($name:ident),+ $(,)?) => {
        $(pub fn $name<T>(a: T, b: T) -> T {
            let _ = (a, b);
            unimplemented!("no-op stub: only for type checking")
        })+
    };
}

binary_fn!(min, max, step, pow, atan2);

/// 三元函数。
pub fn clamp<T>(v: T, lo: T, hi: T) -> T {
    let _ = (v, lo, hi);
    unimplemented!("no-op stub: only for type checking")
}

pub fn mix<T>(a: T, b: T, t: f32) -> T {
    let _ = (a, b, t);
    unimplemented!("no-op stub: only for type checking")
}

pub fn smoothstep<T>(lo: T, hi: T, v: T) -> T {
    let _ = (lo, hi, v);
    unimplemented!("no-op stub: only for type checking")
}

// ---- 向量专用(签名以后收紧成"只接受向量") ----

pub fn dot<T>(a: T, b: T) -> f32 {
    let _ = (a, b);
    unimplemented!("no-op stub: only for type checking")
}

pub fn cross<T>(a: T, b: T) -> T {
    let _ = (a, b);
    unimplemented!("no-op stub: only for type checking")
}

pub fn length<T>(v: T) -> f32 {
    let _ = v;
    unimplemented!("no-op stub: only for type checking")
}

pub fn distance<T>(a: T, b: T) -> f32 {
    let _ = (a, b);
    unimplemented!("no-op stub: only for type checking")
}

pub fn normalize<T>(v: T) -> T {
    let _ = v;
    unimplemented!("no-op stub: only for type checking")
}

pub fn faceforward<T>(n: T, i: T, nref: T) -> T {
    let _ = (n, i, nref);
    unimplemented!("no-op stub: only for type checking")
}

pub fn reflect<T>(i: T, n: T) -> T {
    let _ = (i, n);
    unimplemented!("no-op stub: only for type checking")
}

pub fn refract<T>(i: T, n: T, eta: f32) -> T {
    let _ = (i, n, eta);
    unimplemented!("no-op stub: only for type checking")
}
