//! This module contains all the functions that are used to interact with the database
//! The database is used to store the files that are uploaded

use std::str::FromStr;

use chrono::{DateTime, Local};
use rusqlite::Connection;
use sea_query::{ColumnDef, Expr, Iden, Order, Query, SqliteQueryBuilder, Table};

use crate::{files::get_path, utils::sort_entries};

/// The name of the database
const DB_NAME: &str = "store.db";

/// Establishes a connection to the database
/// The database name is specified in the DB_NAME constant
pub fn connect_to_db() -> Result<Connection, rusqlite::Error> {
    Connection::open(get_path(DB_NAME))
}

#[derive(Iden)]
enum Store {
    Table,
    Id,
    Name,
    Path,
    IsDir,
    AccessedAt,
    CreatedAt,
}

/// Represents a Database Entry
/// It directly reflects the database
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i32,
    pub name: String,
    pub path: String,
    /// Represents if an entry is a dir or not, left for legacy reasons
    #[allow(dead_code)]
    pub is_dir: bool,
    pub accessed_at: DateTime<Local>,
    /// The time the entry was created. Currently not in use anywhere.
    #[allow(dead_code)]
    pub created_at: DateTime<Local>,
}

/// Builder struct that converts to an Entry
#[derive(Debug, Clone)]
pub struct EntryBuilder {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

impl EntryBuilder {
    pub fn new(name: &str, path: &str, is_dir: bool) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            is_dir,
        }
    }
}

/// Prepares the Database, creates all the tables and defines the schema
pub fn prep_db(conn: &Connection) -> rusqlite::Result<usize, rusqlite::Error> {
    let query = Table::create()
        .table(Store::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Store::Id)
                .integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(Store::Name).string().not_null())
        .col(ColumnDef::new(Store::Path).string().not_null())
        .col(ColumnDef::new(Store::IsDir).boolean().not_null())
        .col(ColumnDef::new(Store::AccessedAt).date_time().not_null())
        .col(ColumnDef::new(Store::CreatedAt).date_time().not_null())
        .build(SqliteQueryBuilder);

    conn.execute(&query, [])
}

/// Inserts an entry into the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `eb` - An EntryBuilder struct
///
/// # Returns
/// A Result enum with the following variants:
///
/// * `Entry` - The entry that was inserted into the database
/// * `rusqlite::Error` - The error that was encountered while inserting into the database
pub fn insert_into_db(conn: &Connection, eb: EntryBuilder) -> Result<Entry, rusqlite::Error> {
    let time_now = Local::now().to_string();

    let query = Query::insert()
        .into_table(Store::Table)
        .columns([
            Store::Name,
            Store::Path,
            Store::IsDir,
            Store::AccessedAt,
            Store::CreatedAt,
        ])
        .values_panic([
            eb.name.clone().into(),
            eb.path.clone().into(),
            eb.is_dir.into(),
            time_now.clone().into(),
            time_now.into(),
        ])
        .to_string(SqliteQueryBuilder);

    match does_exist(conn, &eb.path) {
        Ok(entry) => {
            return Ok(entry);
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {}
        Err(_) => {}
    }

    conn.execute(&query, [])
        .expect("Failed to insert into database");

    let query = Query::select()
        .columns([
            Store::Id,
            Store::Name,
            Store::Path,
            Store::IsDir,
            Store::AccessedAt,
            Store::CreatedAt,
        ])
        .from(Store::Table)
        .and_where(Expr::col(Store::Name).eq(eb.name))
        .limit(1)
        .to_string(SqliteQueryBuilder);

    conn.query_row(&query, [], |row| {
        let accessed_at =
            chrono::DateTime::from_str(row.get::<_, String>(4)?.as_str()).unwrap_or(Local::now());
        let created_at =
            chrono::DateTime::from_str(row.get::<_, String>(5)?.as_str()).unwrap_or(Local::now());

        Ok(Entry {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            is_dir: row.get(3)?,
            accessed_at,
            created_at,
        })
    })
}

/// Inserts an entry into the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `e` - An Entry struct
///
/// # Returns
/// A Result enum with the following variants:
///
/// * `Entry` - The entry that was inserted into the database
/// * `rusqlite::Error` - The error that was encountered while inserting into the database
pub fn insert_entry(conn: &Connection, e: Entry) -> Result<usize, rusqlite::Error> {
    let query = Query::insert()
        .into_table(Store::Table)
        .columns([
            Store::Name,
            Store::Path,
            Store::IsDir,
            Store::AccessedAt,
            Store::CreatedAt,
        ])
        .values_panic([
            e.name.clone().into(),
            e.path.clone().into(),
            e.is_dir.into(),
            e.accessed_at.to_string().into(),
            e.created_at.to_string().into(),
        ])
        .to_string(SqliteQueryBuilder);

    conn.execute(&query, [])
}

/// Gets all the entries from the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
///
/// # Returns
/// A Result enum with the following variants:
///
/// * `Vec<Entry>` - A vector of all the entries in the database
/// * `rusqlite::Error` - The error that was encountered while getting the entries from the database
pub fn get_all(conn: &Connection) -> Result<Vec<Entry>, rusqlite::Error> {
    let query = Query::select()
        .columns([
            Store::Id,
            Store::Name,
            Store::Path,
            Store::IsDir,
            Store::AccessedAt,
            Store::CreatedAt,
        ])
        .order_by(Store::Id, Order::Desc)
        .from(Store::Table)
        .to_string(SqliteQueryBuilder);

    let mut stmt = conn.prepare(&query)?;

    let entries = stmt
        .query_map([], |row| {
            let accessed_at = chrono::DateTime::from_str(row.get::<_, String>(4)?.as_str())
                .unwrap_or(Local::now());
            let created_at = chrono::DateTime::from_str(row.get::<_, String>(5)?.as_str())
                .unwrap_or(Local::now());

            Ok(Entry {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                is_dir: row.get(3)?,
                accessed_at,
                created_at,
            })
        })?
        .map(|x| x.unwrap())
        .collect::<Vec<Entry>>();

    Ok(entries)
}

/// Gets an entry from the database
/// using the path of the file
/// essentially checking if the file exists
/// in the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `path` - The path of the file
///
/// # Returns
///
/// A Result enum with the following variants:
///
/// * `Entry` - The entry that was found in the database
/// * `rusqlite::Error` - The error that was encountered while getting the entry from the database
///
/// # Usage
///
/// To essentially check if an entry exists, the Error `rusqlite::Error::QueryReturnedNoRows` is
/// returned if the entry does not exist in the database
/// Otherwise, the entry can be essentially used as a normal entry
pub fn does_exist(conn: &Connection, path: &str) -> Result<Entry, rusqlite::Error> {
    let query = Query::select()
        .columns([
            Store::Id,
            Store::Name,
            Store::Path,
            Store::IsDir,
            Store::AccessedAt,
            Store::CreatedAt,
        ])
        .from(Store::Table)
        .and_where(Expr::col(Store::Path).eq(path))
        .limit(1)
        .to_string(SqliteQueryBuilder);

    conn.query_row(&query, [], |row| {
        let accessed_at =
            chrono::DateTime::from_str(row.get::<_, String>(4)?.as_str()).unwrap_or(Local::now());
        let created_at =
            chrono::DateTime::from_str(row.get::<_, String>(5)?.as_str()).unwrap_or(Local::now());

        Ok(Entry {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            is_dir: row.get(3)?,
            accessed_at,
            created_at,
        })
    })
}

/// Delete an entry from the database
/// using the path of the file
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `path` - The path of the file
///
/// # Returns
///
/// A Result enum with the following variants:
///
/// * `usize` - The number of rows that were deleted
/// * `rusqlite::Error` - The error that was encountered while deleting the entry from the database
pub fn delete_entry(conn: &Connection, path: &str) -> Result<usize, rusqlite::Error> {
    let query = Query::delete()
        .from_table(Store::Table)
        .and_where(Expr::col(Store::Path).eq(path))
        .to_string(SqliteQueryBuilder);

    conn.execute(&query, [])
}

/// Delete all the entries from the database
/// Basically, it drops the table
///
/// Better than deleting all the entries one by one
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
///
/// # Returns
///
/// A Result enum with the following variants:
///
/// * `usize` - The number of rows that were deleted
/// * `rusqlite::Error` - The error that was encountered while deleting the entries from the database
pub fn delete_all(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let table_del = Table::drop()
        .table(Store::Table)
        .if_exists()
        .to_string(SqliteQueryBuilder);

    conn.execute(&table_del, [])?;

    //create the table again
    //so that the program doesn't crash
    //when trying to insert into the database
    prep_db(conn)
}

pub fn pop_one(conn: &Connection) -> Result<Entry, rusqlite::Error> {
    let query = Query::select()
        .columns([
            Store::Id,
            Store::Name,
            Store::Path,
            Store::IsDir,
            Store::AccessedAt,
            Store::CreatedAt,
        ])
        .order_by(Store::Id, Order::Desc)
        .from(Store::Table)
        .limit(1)
        .to_string(SqliteQueryBuilder);

    conn.query_row(&query, [], |row| {
        let accessed_at =
            chrono::DateTime::from_str(row.get::<_, String>(4)?.as_str()).unwrap_or(Local::now());
        let created_at =
            chrono::DateTime::from_str(row.get::<_, String>(5)?.as_str()).unwrap_or(Local::now());

        Ok(Entry {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            is_dir: row.get(3)?,
            accessed_at,
            created_at,
        })
    })
}

pub fn update_accessed_at(conn: &Connection, path: &str) -> Result<usize, rusqlite::Error> {
    let time_now = Local::now().to_string();

    let query = Query::update()
        .table(Store::Table)
        .values([(Store::AccessedAt, time_now.into())])
        .and_where(Expr::col(Store::Path).eq(path))
        .to_string(SqliteQueryBuilder);

    conn.execute(&query, [])
}

pub fn reid(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let mut entries = get_all(conn)?;
    sort_entries(&mut entries);
    entries.reverse();

    delete_all(conn)?;

    for e in entries {
        insert_entry(conn, e)?;
    }

    Ok(0)
}
