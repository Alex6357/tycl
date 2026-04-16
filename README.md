# Typed Configuration Language (TyCL)

TyCL 是一门专为现代软件工程设计的**强类型配置与数据序列化语言**。

它诞生于对现有配置格式（如 JSON 的死板、YAML 的缩进歧义、TOML 的长键冗余）的反思。TyCL 结合了 **JSON 的嵌套直观性**、**Rust 的 Raw 字符串体验**、**Scala 的对齐多行字符串**，并首次在配置领域引入了原生的**渐进式类型注解系统**。

## 🌟 核心特性

1. **内建静态类型与 Schema 校验**：直接在配置中声明结构，支持基本类型、复杂容器与可空标识 (`?`)。
2. **拒绝过度压缩歧义**：顶层键值对严格要求换行分割，代码清晰易读。
3. **JSON 风格的嵌套表**：抛弃 TOML 冗长的 `[a.b.c.d]` 前缀，回归直观的 `{}` 树状作用域。
4. **地表最强字符串系统**：5 种字符串模式任君选择，彻底解决代码缩进破坏字符串内容的痛点。
5. **原生时间支持**：开箱即用的 ISO 8601 时间类型。
6. **Rust 生态原生集成**：提供 CLI 工具、Schema 驱动的 Rust 代码生成、以及 `TryFromValue` Derive 宏。

---

## 📦 项目结构

本仓库是一个 Rust Workspace，包含以下 Crate：

| Crate                               | 说明                                                                                                       |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| [`tycl_parser`](crates/tycl_parser) | 核心解析器。支持文档解析、Schema 解析、带 Schema 的校验解析，以及可选的 `serde` 集成。                     |
| [`tycl_macro`](crates/tycl_macro)   | 提供 `#[derive(TryFromValue)]` 过程宏，自动为 Rust 结构体/枚举实现从 `tycl_parser::Value` 的类型安全转换。 |
| [`tycl_cli`](crates/tycl_cli)       | 命令行工具 `tycl`，提供格式转换 (`convert`) 与 Rust 代码生成 (`generate`) 功能。                           |

---

## 🛠️ CLI 工具

安装后，你可以通过 `tycl` 命令与 TyCL 交互：

### `tycl convert` — 格式转换

将 TyCL 配置文件转换为 JSON、TOML 或 YAML：

```bash
tycl convert config.tycl --format json
tycl convert config.tycl --schema config.schema.tycl --format yaml -o config.yaml
```

### `tycl generate` — Rust 代码生成

根据 Schema 自动生成类型安全的 Rust 数据结构：

```bash
tycl generate config.schema.tycl --root Config -o config_gen.rs
```

Schema 中可通过 `$root-name` 指定根结构体名称，或通过 `--root` 参数覆盖。

---

## 📖 语法速览

### 1. 基础与类型注解

类型注解是可选的，但强烈建议在重要节点使用。顶层的键值对**必须以换行符分隔**。

```tycl
// 基础声明
project_name: str = "tycl_parser"
version: str? = null       // 支持可空类型 (?)
max_workers: int = 16_000  // 支持数字下划线
```

### 2. 容器：Map、List、Record 与 Tuple

在 `map` 和 `list` 内部，不再强制要求换行，元素之间**严格使用逗号 `,` 分隔**。原生支持**尾随逗号**，对 Git Diff 极其友好。

```tycl
endpoints: list(str) = [ "/api", "/graphql", ]

database: record(Database)(
    host: str = "",
    port: int = 0,
) = {
    host = "127.0.0.1",
    port = 5432,
}

pair: tuple(int, str) = (42, "answer")
```

Record 和 Tuple 支持显式命名（如 `record(Database)`），这在 Schema 驱动的 Rust 代码生成中是必需的。

### 3. 枚举与环境变量

```tycl
status: enum(Status)("active", "inactive") = "active"

token: env("API_TOKEN", str) = "default-token"
```

枚举元素和字段名均支持通过 `(alias)` 语法指定映射名称，方便 Rust 代码生成时的命名转换：

```tycl
log_level: enum("debug" (dbg), "info", "warn" (warning), "error") = "info"
```

### 4. 五大字符串魔法 🪄

TyCL 拥有目前配置语言中最极致的字符串处理体验。

#### 模式 A：普通字符串与多行字符串

```tycl
basic: str = "Hello \\n World"
multi: str = """
这首行的换行会被自动忽略。
这里的真实换行会被保留。
"""
```

#### 模式 B：字面量 (Raw) 字符串

受 Rust 启发，无需任何转义，极其适合书写正则表达式或内嵌代码。通过 `#` 的数量来界定边界。

```tycl
regex: str = r#"^[a-z]+(?:/[a-z]+)*$"#
json_str: str = r##"{"key": "value"}"##
```

#### 模式 C：多行对齐字符串 (`|"""`) ✨ [杀手级特性]

在传统的配置语言中，多行字符串的缩进往往会破坏内容原有的格式。TyCL 引入了 `|` 触发器，不仅维持了代码缩进的美观，还能精确控制逻辑换行：

```tycl
query: str = |"""
             |SELECT id, name
             |FROM users
             |WHERE age > 18
             |"""
```

**解析结果：** `SELECT id, name\nFROM users\nWHERE age > 18\n`
_(解析器会自动剥离 `|` 之前的所有空白字符!)_

#### 模式 D：多行对齐字面量 (`r|#"`)

结合了 Raw 字符串的不转义特性和对齐字符串的缩进控制，是内嵌大型脚本的终极方案：

```tycl
lua_script: str = r|#"
                  |function setup()
                  |    print("Absolutely no \\n escape needed!")
                  |end
                  |"#
```

### 5. 第一公民的时间类型

直接书写符合 ISO 8601 规范的时间，无需加引号，类型安全。

```tycl
created_at: time(datetime) = 2024-05-20T10:30:00
updated_at: time(offset)   = 2024-05-20T10:30:00+08:00
birthday: time(localdate)  = 1990-01-01
```

---

## 🔧 Rust 集成

### `TryFromValue` Derive 宏

`tycl_macro` 提供的 Derive 宏可以将解析后的 `Value` 自动转换为你的 Rust 类型：

```rust
use tycl_parser::TryFromValue;

#[derive(Debug, TryFromValue)]
struct Database {
    host: String,
    port: i64,
}

#[derive(Debug, TryFromValue)]
#[tycl(rename = "app_state")]
struct AppState {
    name: String,
    database: Database,
}
```

### Serde 集成

启用 `tycl_parser` 的 `serde` feature 后，可直接将 `Document` 与 `Value` 和 `serde_json` / `serde_yaml` / `toml` 互操作：

```rust
let doc: Document = tycl_parser::parse(source)?;
let json = serde_json::to_string_pretty(&doc)?;
```

---

## 🛠️ 设计哲学 (Why TyCL?)

1. **Why 强制顶层换行？**
   允许 `a=1 b=2` 在同一行会导致肉眼解析困难，TyCL 强制要求顶层 KV 换行，以保证配置文件的绝对可读性。
2. **Why 容器内严格使用逗号？**
   YAML 中缩进决定层级的做法容易导致线上事故。TyCL 在大括号嵌套中强制使用逗号 `,`，让数据结构的边界像 JSON 一样绝对清晰。
3. **Why 引入类型？**
   在微服务时代，配置即契约（Config as a Contract）。类型前置不仅能让 IDE 提供精准提示，还能在启动阶段依靠 Schema 将错误拦截，避免运行时的类型转换异常。
