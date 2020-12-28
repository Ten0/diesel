#[cfg(feature = "uses_information_schema")]
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSqlRow, FromStaticSqlRow};
#[cfg(feature = "uses_information_schema")]
use diesel::sql_types;
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;

#[cfg(feature = "uses_information_schema")]
use super::information_schema::UsesInformationSchema;
use super::table_data::TableName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInformation {
    pub column_name: String,
    pub type_name: String,
    pub nullable: bool,
}

#[derive(Debug)]
pub struct ColumnType {
    pub rust_name: String,
    pub is_array: bool,
    pub is_nullable: bool,
    pub is_unsigned: bool,
}

use std::fmt;

impl fmt::Display for ColumnType {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.is_nullable {
            write!(out, "Nullable<")?;
        }
        if self.is_array {
            write!(out, "Array<")?;
        }
        if self.is_unsigned {
            write!(out, "Unsigned<")?;
        }
        write!(out, "{}", self.rust_name)?;
        if self.is_unsigned {
            write!(out, ">")?;
        }
        if self.is_array {
            write!(out, ">")?;
        }
        if self.is_nullable {
            write!(out, ">")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ColumnDefinition {
    pub sql_name: String,
    pub rust_name: String,
    pub ty: ColumnType,
    pub docs: String,
}

impl ColumnInformation {
    pub fn new<T, U>(column_name: T, type_name: U, nullable: bool) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        ColumnInformation {
            column_name: column_name.into(),
            type_name: type_name.into(),
            nullable,
        }
    }
}

#[cfg(feature = "uses_information_schema")]
impl<DB> FromSqlRow<(sql_types::Text, sql_types::Text, sql_types::Text), DB> for ColumnInformation
where
    DB: Backend + UsesInformationSchema,
    (String, String, String):
        FromStaticSqlRow<(sql_types::Text, sql_types::Text, sql_types::Text), DB>,
{
    fn build_from_row<'a>(row: &impl diesel::row::Row<'a, DB>) -> deserialize::Result<Self> {
        let row = <(String, String, String) as FromStaticSqlRow<
            (sql_types::Text, sql_types::Text, sql_types::Text),
            DB,
        >>::build_from_row(row)?;
        Ok(ColumnInformation::new(row.0, row.1, row.2 == "YES"))
    }
}

#[cfg(feature = "sqlite")]
impl<ST> FromSqlRow<ST, Sqlite> for ColumnInformation
where
    (i32, String, String, bool, Option<String>, bool): FromStaticSqlRow<ST, Sqlite>,
{
    fn build_from_row<'a>(row: &impl diesel::row::Row<'a, Sqlite>) -> deserialize::Result<Self> {
        let row = <(i32, String, String, bool, Option<String>, bool) as FromStaticSqlRow<
            ST,
            Sqlite,
        >>::build_from_row(row)?;
        Ok(ColumnInformation::new(row.1, row.2, !row.3))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForeignKeyConstraint {
    pub child_table: TableName,
    pub parent_table: TableName,
    pub foreign_key: String,
    pub foreign_key_rust_name: String,
    pub primary_key: String,
}

impl ForeignKeyConstraint {
    pub fn ordered_tables(&self) -> (&TableName, &TableName) {
        use std::cmp::{max, min};
        (
            min(&self.parent_table, &self.child_table),
            max(&self.parent_table, &self.child_table),
        )
    }
}
