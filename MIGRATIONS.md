# Database Migrations

## Overview

Media Shelf uses SQLite (`media-shelf.sqlite3`) stored in the user's app data directory.
Schema changes after this release **must be handled via migration** — do not modify `schema.sql` alone.

## Migration system

Migrations are applied in `src-tauri/src/db.rs` via the `migrate_media_table()` function.
It uses `PRAGMA table_info(media)` to detect existing columns and `ALTER TABLE ADD COLUMN` for missing ones.

### Current migration pattern

```rust
fn migrate_media_table(connection: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get current column names
    let mut column_names = Vec::new();
    let mut statement = connection.prepare("PRAGMA table_info(media)")?;
    // ...

    // 2. Add missing columns
    if !column_names.iter().any(|column| column == "new_column") {
        connection.execute("ALTER TABLE media ADD COLUMN new_column TEXT", [])?;
    }

    // 3. Create indexes if needed
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_new ON media(new_column)",
        [],
    )?;

    Ok(())
}
```

## Schema history

| Version | Columns added | Notes |
|---------|--------------|-------|
| Initial (v0.1.0) | `id`, `file_path`, `source_path`, `file_name`, `file_type`, `mime_type`, `width`, `height`, `duration_seconds`, `file_size`, `file_hash`, `thumbnail_path`, `imported_at`, `created_at`, `metadata_json` | Base schema |
| v0.1.0 (patch) | `source_url`, `source_title`, `thumbnail_path` (migration) | URL title fetching for comics |
| v0.1.0 (patch) | `metadata_json` (migration) | Comic page metadata storage |
| **v0.2.0** | `series_name`, `completed_at`, `last_viewed_at` | Series grouping, read completion tracking |

## Rules for future schema changes

1. **Always add migration logic** in `migrate_media_table()` — do not rely on `schema.sql` alone for existing databases.
2. **Use `PRAGMA table_info`** to check if a column exists before adding it.
3. **`ALTER TABLE ADD COLUMN`** is safe for adding nullable columns. For non-nullable columns with defaults, use `DEFAULT` clause.
4. **Update `schema.sql`** to reflect the final desired schema (for fresh installs).
5. **Update `MediaRecord`** struct in `media_scan.rs` to include new fields.
6. **Update `MediaItem`** interface in `types.ts` for TypeScript type safety.
7. **Test with existing databases** — the migration must work on databases created by previous versions.
8. **For complex changes** (renaming columns, changing types, adding foreign keys), consider creating a new table and migrating data, since SQLite has limited `ALTER TABLE` support.

## Tables

### media
Core media item table. Stores images, videos, and comic archives.

### tag_categories
Categories for organizing tags (人物, 場所, 作品, etc.).

### tags
Individual tags belonging to a category.

### media_tags
Many-to-many relationship between media and tags.

### notes
Free-form notes attached to media items.
