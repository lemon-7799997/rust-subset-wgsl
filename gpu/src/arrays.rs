// ============================================================================
// array<T>:假容器(桩库规则:只实现"必须的功能")
// 只放一个 phantom 元素,Index/IndexMut 忽略下标——目的纯粹是让重放副本
// 能过 rustc 类型检查;真正的数组是 WGSL 侧的 storage buffer 成员。
// WGSL 数组下标是 u32/i32,所以支持 usize/u32 下标,让源写法贴近 WGSL。
// 元素类型可以是 glam 向量(如 array<Vec4>),只要 T: ConstDefault。
// ============================================================================

use crate::ConstDefault;

#[derive(gpu_macro::ConstDefault)]
pub struct array<T: ConstDefault> {
    phantom: T,
}

impl<T: ConstDefault> array<T> {}

impl<T: ConstDefault> core::ops::Index<usize> for array<T> {
    type Output = T;
    #[inline]
    fn index(&self, _i: usize) -> &T {
        &self.phantom
    }
}

impl<T: ConstDefault> core::ops::IndexMut<usize> for array<T> {
    #[inline]
    fn index_mut(&mut self, _i: usize) -> &mut T {
        &mut self.phantom
    }
}

// WGSL 风格:直接用 u32 下标(不需要 `as usize`)
impl<T: ConstDefault> core::ops::Index<u32> for array<T> {
    type Output = T;
    #[inline]
    fn index(&self, _i: u32) -> &T {
        &self.phantom
    }
}

impl<T: ConstDefault> core::ops::IndexMut<u32> for array<T> {
    #[inline]
    fn index_mut(&mut self, _i: u32) -> &mut T {
        &mut self.phantom
    }
}
