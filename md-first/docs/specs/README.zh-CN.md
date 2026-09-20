---
doc_id: md-first.specs.core
title: "Specification"
language: zh-CN
source_language: zh-CN
counterpart: ./README.md
implementation_status: current
document_status: draft
translation_status: pending
last_verified: 2026-09-20
---

# Specification

- [Configuration](#configuration)
- [Dispatcher](#dispatcher)
- [Preprocessor](#preprocessor)
- [Rust header](#rust-header)
- [自举验收](#自举验收)

## Configuration

配置写在 project root 的 `Cargo.toml`：

```toml
[workspace.metadata.md-first]
include = ["md-first/docs/specs/README.zh-CN.md"]
exclude = []
```

`include`、`exclude` 使用相对 project root 的 glob；`exclude` 优先，同一路径只扫描一次。
只处理标注 `out=输出路径` 且显式闭合的 fenced code block。`out` 相对 project root，不能使用绝对路径或越出根目录。

## Dispatcher

```rust out=md-first/crates/core/src/dispatcher.h.rs
use std::path::{Path, PathBuf};
use crate::{Error, Preprocessor};

#[derive(Default)]
#[spec::private_fields]
pub struct Dispatcher {}

#[spec::header]
impl Dispatcher {
    /// 创建空的 Preprocessor 注册表。
    pub fn new() -> Self;

    /// 每个 out 选择匹配后缀最长的 Preprocessor。
    /// - error: 后缀为空或已注册。
    pub fn register(
        &mut self,
        suffix: impl Into<String>,
        preprocessor: impl Preprocessor + 'static,
    ) -> Result<(), Error>;

    /// 读取配置，生成并覆盖各 out；自动创建父目录，返回按路径排序的项目相对输出路径。
    /// 文件顶部包含 DO NOT EDIT 提示及来源 Markdown 路径，来源按输入顺序去重。
    /// - error: 配置、读取、分词或预处理失败；out 没有匹配的 Preprocessor。
    ///   这些错误发生时不写入任何输出；写入阶段的 I/O 错误不保证整体回滚。
    pub fn generate(&self, project_root: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error>;

    /// 在内存中重新生成并比较文件内容，全程不写入。
    /// - error: 生成失败，或输出文件缺失、内容不同。
    pub fn check(&self, project_root: impl AsRef<Path>) -> Result<(), Error>;
}
```

## Preprocessor

```rust out=md-first/crates/core/src/preprocessor.h.rs
use std::path::PathBuf;
use proc_macro2::{Span, TokenStream};

/// # Examples
///
/// ```
/// use md_first_core::{Context, Dispatcher, Preprocessor};
/// use proc_macro2::TokenStream;
///
/// // 1. Define how blocks with the same out are combined
/// struct Merge;
/// impl Preprocessor for Merge {
///     fn preprocess(&self, blocks: Vec<TokenStream>, _context: &Context) -> TokenStream {
///         blocks.into_iter().collect()
///     }
/// }
///
/// // 2. Register the preprocessor for an output suffix
/// let mut dispatcher = Dispatcher::new();
/// dispatcher.register(".rs", Merge)?;
/// # Ok::<(), md_first_core::Error>(())
/// ```
pub trait Preprocessor {
    /// 同一 out 的 blocks 一次传入；跨文档按项目相对路径排序，文档内按出现顺序排列。
    /// 每个 block 独立分词，分隔符必须配对，无需包含完整 Rust item。
    /// 解析、合并及跨文档声明是否合法，由 Preprocessor 决定；返回值序列化后写入 out。
    fn preprocess(&self, blocks: Vec<TokenStream>, context: &Context) -> TokenStream;
}

#[spec::private_fields]
pub struct Context {
    /// 绝对路径。
    pub project_root: PathBuf,
    /// 相对 project_root。
    pub out: PathBuf,
    /// `sources[i]` 对应 `blocks[i]`。
    pub sources: Vec<BlockSource>,
}

#[spec::header]
impl Context {
    /// span 来自 `blocks[block_index]`；错误定位到原始 Markdown，本轮生成失败且不写入输出。
    pub fn error(&self, block_index: usize, span: Span, message: impl Into<String>);
}

#[derive(Clone, Debug)]
pub struct BlockSource {
    /// 相对 project_root。
    pub document: PathBuf,
    /// 代码内容在原始 Markdown 中的起始行，1-based；精确诊断使用 Context::error。
    pub start_line: usize,
}
```

## Rust header

`md_first_preprocessor::rust_header::RustHeader` 按输入顺序合并 blocks；命令入口将其注册到 `.h.rs`。
除下列规则外，保留标准 Rust 语法与 attributes。

| 声明 | 规则 |
| --- | --- |
| struct 字段 | 显式 `pub`；`#[spec::private_fields]` 生成的字段除外。 |
| `#[spec::private_fields]` | 具名字段 struct 增加私有 `inner: TypeInner`；泛型实参随类型传递，`TypeInner` 由实现方定义。已有 `inner` 字段时报错。 |
| `#[spec::header]` | inherent impl 中的方法显式 `pub`，以 `;` 省略方法体；签名与 Rustdoc 来自声明，方法体转发到同类型的 `方法名_impl`。 |

声明：

```rust
#[spec::private_fields]
pub struct Counter {
    pub name: String,
}

#[spec::header]
impl Counter {
    pub fn value(&self) -> u64;
}
```

生成：

```rust
pub struct Counter {
    pub name: String,
    inner: CounterInner,
}

impl Counter {
    pub fn value(&self) -> u64 {
        Self::value_impl(self)
    }
}
```

转发保留类型与 const 泛型参数；`async` 等待实现结果，`unsafe` 调用放在显式 unsafe block 中。
解构、`ref` 和 `_` 参数传递完整实参；C variadic 方法无法自动转发，在 header 中报错。

## 自举验收

1. 从已提交的生成代码编译 md-first，读取本文，生成自己的接口。
2. 使用刚生成的接口重新编译 md-first，再读取本文生成一次。
3. 两次编译都成功，且两轮输出逐字节一致，才通过。

生成文件与 Spec 一同提交到 Git；CI 还需检查提交的文件与生成结果一致。
