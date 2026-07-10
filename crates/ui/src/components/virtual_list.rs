//! VirtualList 组件封装 —— 基于 gpui-component 的 VirtualList
//!
//! 虚拟列表组件，用于渲染大量不同尺寸的行/列。仅渲染可见范围，性能优异。
//! 构造器为函数 `v_virtual_list` / `h_virtual_list`，非 `VirtualList::new(id)`。
//!
//! ## 声明式语法
//!
//! ```rml
//! <virtual-list direction="vertical" item-sizes={item_sizes}>
//!     <template slot="render" each={i in range}>
//!         <div>{items[i].name}</div>
//!     </template>
//! </virtual-list>
//! ```

pub use gpui_component::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
