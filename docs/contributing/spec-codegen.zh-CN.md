---
doc_id: contributing.spec-codegen
title: "从 Markdown 生成 Rust 头文件"
language: zh-CN
source_language: en
counterpart: ./spec-codegen.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-17
---

# 从 Markdown 生成 Rust 头文件

[English](spec-codegen.md)

`spec-codegen` 将选定 Markdown 文档中的声明生成 Rust 头文件。接口声明由一份规范源文档维护，
私有状态和方法实现留在手写 Rust 中。现有 Engine 和 builtin 声明尚未迁移。

## 标记需要生成的声明

只使用一个 `header=<snake_case_name>` 属性。同一文档中使用相同 key 的多个代码块，按出现顺序
生成到 `<name>.h.rs`。只选择规范源文档，不要同时选择它的翻译。

````markdown
<details>
<summary>Imports</summary>

```rust header=counter
use std::string::String;
```

</details>

```rust header=counter
pub struct Counter {
    pub label: String,
}
```

```rust header=counter
impl Counter {
    pub fn new(label: String) -> Self;
    pub fn value(&self) -> u64;
}
```
````

`<summary>` 后保留一个空行，让 Markdown 将 fence 解析为代码块，而不是原始 HTML。
引用块和列表内也可以放置代码块。每个被标记的 fence 都必须显式闭合，并包含完整声明。
未标记的 fence 不参与生成；用于展示 Markdown 示例的外层 fence 也不会被递归提取。

```text
use / enum / type / ordinary struct → preserve declaration
struct + bodyless inherent impl    → inject private inner: <Type>Inner
receiver method                   → self.<method>_impl(arguments)
associated function               → Self::<method>_impl(arguments)
```

生成器先收集全部代码块，再识别声明关系，因此方法可以出现在 struct 之前。允许拆成多个 impl 块，
但不允许重复声明或重复方法。同一个 header key 不能属于两份输入文档。

生成的结构体保留公开字段：

```rust
pub struct Counter {
    pub label: String,    // Declared visibility is preserved.
    inner: CounterInner, // Injected private storage.
}
```

在 `counter.rs` 中手写实现：

```rust
use spec_header::include_header;

include_header!(counter); // Same module, not a nested module.

struct CounterInner {
    value: u64,
}

impl Counter {
    fn new_impl(label: String) -> Self {
        Self { label, inner: CounterInner { value: 0 } }
    }

    fn value_impl(&self) -> u64 {
        self.inner.value
    }
}
```

调用方可以直接读写 `counter.label`，不能访问 `inner`，也不能在模块外通过 struct literal 构造实例。
生成器不初始化字段、不要求实现 `Default`，也不生成业务逻辑。每个使用模块只 include 同一个头文件一次。

## 接入构建

对于位于 workspace 顶层目录的消费方 crate：

```toml
[dependencies]
spec_header = { path = "../tools/spec-codegen/header-support" }

[build-dependencies]
spec-codegen = { path = "../tools/spec-codegen" }
```

把规范源文件放在消费方 package 内，例如 `spec/api.md`。在它的 `build.rs` 中：

```rust
fn main() {
    println!("cargo:rerun-if-changed=spec/api.md");
    spec_codegen::generate(
        &["spec/api.md"], // Pass all canonical inputs in one invocation.
        std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"),
    ).expect("generate Rust headers"); // Failure must stop the build.
}
```

每份源文件都要注册 `rerun-if-changed`。确保打包规则保留这些输入，可用 `cargo package --list` 检查。
宏在消费方编译上下文中展开，只负责 include `OUT_DIR/<key>.h.rs`。
普通依赖 `spec_header` 使用 `no_std`，且没有任何依赖；解析器只作为构建期依赖存在。

CLI 使用同一套库逻辑，输出目录由调用方指定：

```sh
cargo run -p spec-codegen -- --out-dir target/spec-headers path/to/api.md
just test-spec-codegen
```

## 明确检查范围

- 普通数据声明可以带泛型；门面必须是非泛型、使用命名字段的 struct。
- 方法支持 `self`、`&self`、`&mut self` 或无 receiver，参数必须有名称。
- 保留 `doc` 和 `derive` 属性；拒绝不支持的属性。
- 拒绝 trait impl、方法体、门面泛型方法（包括参数位置的 `impl Trait`）、显式类型 receiver、解构参数，以及
  `async`/`const`/`unsafe`/`extern`/可变参数方法。
  返回位置的 `impl Trait` 不引入泛型参数，予以保留。
- `inner`、`<Type>Inner` 和 `*_impl` 保留给门面的私有状态及手写实现。
- 缺少实现方法或类型不兼容会导致 Rust 编译失败。行为正确性、额外手写的公开方法仍需要测试和 review。

生成诊断会报告输入路径和 Markdown 行号。头文件记录声明及方法的来源，不包含时间戳或机器相关的绝对路径。
Rust 编译错误可能指向生成的头文件，不会自动映射回 Markdown。

输出目录中的 `.spec-codegen-manifest` 记录工具拥有的头文件。成功执行时，key 被删除或更名会清理
对应旧文件，其它文件保持不变。已存在但不属于工具的文件、非法清单，以及输出文件位置上的符号链接都会
被拒绝。不要手改生成文件，也不要向同一个输出目录并发执行多次生成。

全部输入通过校验后才开始写入；每个文件的替换是原子的，但整个文件集合不是一个事务。
必须向上传播所有失败：失败后仍存在的旧文件不能作为回退结果使用。

[生成器入口](../../tools/spec-codegen/src/lib.rs)和
[Cargo 消费方测试](../../tools/spec-codegen/tests/cargo_consumer.rs)提供实现与端到端接入示例。
builtin 目录声明的迁移仍是后续独立工作。
