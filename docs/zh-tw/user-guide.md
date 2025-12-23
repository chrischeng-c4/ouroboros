# 使用指南 (User Guide)

`data-bridge` 是一個為 Python 設計的高效能 MongoDB ORM，其核心由 Rust 驅動。它提供與 Beanie 相容的 API，同時將所有 BSON 序列化和 CPU 密集型任務交由 Rust 處理，從而顯著提升效能。

## 快速開始 (Getting Started)

首先，初始化與 MongoDB 實例的連線。這通常在應用程式啟動時完成。

```python
import asyncio
from data_bridge import init

async def main():
    # 使用連線字串和資料庫名稱進行初始化
    await init("mongodb://localhost:27017/my_database")

if __name__ == "__main__":
    asyncio.run(main())
```

## 定義模型 (Defining Models)

透過繼承 `Document` 來定義模型。您可以使用標準的 Python 型別提示 (Type Hints)。

```python
from typing import Optional
from data_bridge import Document, Indexed

class User(Document):
    name: str
    email: Indexed(str, unique=True)  # 建立唯一索引
    age: int = 0
    is_active: bool = True
    
    class Settings:
        name = "users"  # 集合名稱 (Collection name)
```

### 設定配置 (Settings Configuration)
`Settings` 內部類別用於配置模型：

*   `name`: 集合名稱 (預設為類別名稱的小寫)
*   `indexes`: 索引定義列表
*   `use_revision`: 透過 `_revision_id` 啟用樂觀鎖 (Optimistic Locking)
*   `is_root`: 標記為文件繼承的根類別

## CRUD 操作

### 建立 (Create)

建立一個新的文件實例並儲存。

```python
user = User(name="Alice", email="alice@example.com", age=30)
await user.save()
```

### 讀取 (Read)

透過 ID 或其他條件尋找文件。

```python
# 透過 ID 尋找
user = await User.get("507f1f77bcf86cd799439011")

# 透過欄位尋找單筆
user = await User.find_one(User.email == "alice@example.com")
```

### 更新 (Update)

修改欄位並儲存變更。

```python
user.age = 31
await user.save()

# 使用查詢直接更新 (無需先取出)
await User.find(User.name == "Alice").update({"$set": {"age": 32}})
```

### 刪除 (Delete)

刪除文件實例或符合條件的文件。

```python
# 刪除實例
await user.delete()

# 透過查詢刪除
await User.find(User.is_active == False).delete()
```

## 查詢 (Querying)

`data-bridge` 支援流暢且可串接的查詢 API，並提供型別安全的表達式。

### 基礎過濾

```python
# 精確匹配
users = await User.find(User.age == 30).to_list()

# 比較運算符
users = await User.find(User.age > 25).to_list()
users = await User.find(User.age <= 50).to_list()

# 多重條件 (AND)
users = await User.find(
    User.age > 25,
    User.is_active == True
).to_list()
```

### 排序、跳過與限制 (Sorting, Skipping, and Limiting)

```python
users = await User.find(User.is_active == True) \
    .sort(-User.age) \
    .skip(10) \
    .limit(20) \
    .to_list()
```

*   `.sort(+User.field)`: 升冪 (Ascending)
*   `.sort(-User.field)`: 降冪 (Descending)

### 投影 (Projections)

僅讀取特定欄位以節省頻寬。

```python
# 僅包含 name 和 email
users = await User.find().project(name=1, email=1).to_list()
```

## 批次操作 (Bulk Operations)

使用流暢的批次 API 高效執行多個寫入操作。所有操作皆在 Rust 中處理。

```python
from data_bridge import UpdateOne, InsertOne, DeleteOne

await User.bulk_write([
    # 插入新使用者
    InsertOne(User(name="Bob", email="bob@example.com")),
    
    # 更新現有使用者
    UpdateOne(User.email == "alice@example.com")
        .set(User.status, "vip")
        .inc(User.login_count, 1),
        
    # 刪除不活躍的使用者
    DeleteOne(User.last_login < "2023-01-01")
])
```

## 進階模型 (Advanced Models)

### 嵌入式文件 (Embedded Documents)

您可以使用 `EmbeddedDocument` 在文件內嵌套其他文件。與 `Document` 不同，這些文件沒有自己獨立的集合 (Collection)。

```python
from data_bridge import Document, EmbeddedDocument

class Address(EmbeddedDocument):
    city: str
    zip_code: str
    street: str | None = None

class User(Document):
    name: str
    address: Address

    class Settings:
        name = "users"

# 使用方式
user = User(
    name="Alice",
    address=Address(city="NYC", zip_code="10001")
)
await user.save()
```

### 約束與驗證 (Constraints and Validation)

`data-bridge` 支援使用 `typing.Annotated` 進行欄位級別的驗證。驗證是在 Rust 後端執行的，以確保高效能。

```python
from typing import Annotated, Optional
from data_bridge import Document, MinLen, MaxLen, Min, Max, Email, Url

class Product(Document):
    name: Annotated[str, MinLen(3), MaxLen(100)]
    price: Annotated[float, Min(0.0)]
    contact_email: Annotated[str, Email()]
    website: Annotated[Optional[str], Url()] = None

    class Settings:
        use_validation = True # 儲存時啟用驗證
```

---

## 關聯 (Relations / Links)

`data-bridge` 提供與 Beanie 相容的文件連結功能。

### 一對一 / 多對一
使用 `Link[T]` 引用另一個文件。

```python
from data_bridge import Document, Link

class User(Document):
    name: str

class Post(Document):
    title: str
    author: Link[User]

# 建立連結
user = await User.find_one(User.name == "Alice")
post = Post(title="Hello World", author=user)
await post.save()

# 讀取並解析連結
post = await Post.find_one(Post.title == "Hello World", fetch_links=True)
print(post.author.name) # "Alice"
```

### 一對多
使用 `BackLink[T]` 定義反向關聯。

```python
from data_bridge import Document, BackLink

class User(Document):
    name: str
    # 引用指向此使用者的 Posts
    posts: BackLink["Post"] = BackLink(document_class="Post", link_field="author")

# 存取方式
user = await User.find_one(User.name == "Alice", fetch_links=True)
for post in user.posts:
    print(post.title)
```

---

## 程式化遷移 (Programmatic Migrations)

`data-bridge` 支援程式化遷移，讓您的資料庫模式 (Schema) 隨時間演進。

```python
from data_bridge.migrations import Migration, iterative_migration, run_migrations

@iterative_migration(User, batch_size=50)
class NormalizeEmails(Migration):
    version = "001"
    description = "將所有電子郵件地址轉為小寫"

    async def transform(self, user: User) -> User:
        user.email = user.email.lower()
        return user

# 執行所有待處理的遷移
await run_migrations([NormalizeEmails])
```

---

## 時序集合 (Time-Series Collections)

對於高頻率資料，可以使用 MongoDB 原生的時序集合。

```python
from datetime import datetime
from data_bridge import Document
from data_bridge.timeseries import TimeSeriesConfig, Granularity

class Measurement(Document):
    timestamp: datetime
    sensor_id: str
    value: float

    class Settings:
        name = "measurements"
        timeseries = TimeSeriesConfig(
            time_field="timestamp",
            meta_field="sensor_id",
            granularity=Granularity.seconds,
            expire_after_seconds=86400 * 7 # 7 天過期 (TTL)
        )
```

---

## HTTP 客戶端 (HTTP Client)

本函式庫包含一個由 Rust (`reqwest`) 支援的高效能非同步 HTTP 客戶端，它能繞過 GIL 以獲得最大吞吐量。

```python
from data_bridge.http import HttpClient

client = HttpClient(
    base_url="https://api.example.com",
    timeout=30.0
)

# 非同步 GET 請求
response = await client.get("/users/123")

if response.is_success():
    data = response.json()
    print(f"使用者: {data['name']}")
    print(f"延遲: {response.latency_ms}ms")
```
