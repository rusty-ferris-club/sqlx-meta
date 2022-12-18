# Sqlx Meta

A library derived as a slim version of [sqlx-crud](https://github.com/treydempsey/sqlx-crud) to provide metadata about the entity it marks, design to only provide the building blocks for you to build your own queries.


## Getting started

Add to your dependencies:

```toml
sqlx-meta = "0.1.0"
```

And mark your entity with `SqlxMeta`:

```rust
#[derive(Default, Debug, Clone, sqlx::FromRow, SqlxMeta)]
pub struct File {
    pub id: Option<i32>,
    pub entry_time: String,
    pub abs_path: String,
    pub path: String,
    pub ext: Option<String>,
    pub mode: Option<String>,
    // ..
}
```

Build your own query (prefer `lazy_static` when it doesn't really change):

```rust
lazy_static! {
    static ref INSERT_SQL: String = {
        let cols = &File::columns()[1..];

        let holders = (0..cols.len()).map(|_| "?").collect::<Vec<_>>().join(", ");

        let excludes = cols
            .iter()
            .map(|c| format!("'{}'=excluded.'{}'", c, c))
            .collect::<Vec<_>>()
            .join(",");
        // ..
        // ..
```

Use the `update_binds` or `insert_binds` methods to save up typing:

```rust
use sqlx_meta::{Binds, Schema};
f.update_binds(q).fetch_optional(&mut conn).await?;
```
