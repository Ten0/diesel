use super::result::PgResult;
use super::row::PgRow;
use super::PgConnection;
use std::cell::RefCell;
use std::rc::Rc;

#[allow(missing_debug_implementations)]
pub struct Cursor<'conn> {
    current_row: usize,
    inner: Rc<PgResultAndConn<'conn>>,
}

pub(super) struct PgResultAndConn<'conn> {
    pub(super) db_result: PgResult,
    pub(super) conn: RefCell<&'conn mut PgConnection>,
}

impl<'conn> Cursor<'conn> {
    pub(super) fn new(conn: &'conn mut PgConnection, result: PgResult) -> Self {
        Self {
            current_row: 0,
            inner: Rc::new(PgResultAndConn {
                db_result: result,
                conn: RefCell::new(conn),
            }),
        }
    }
}

impl ExactSizeIterator for Cursor<'_> {
    fn len(&self) -> usize {
        self.inner.db_result.num_rows() - self.current_row
    }
}

impl<'conn> Iterator for Cursor<'conn> {
    type Item = crate::QueryResult<PgRow<'conn>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row < self.inner.db_result.num_rows() {
            let row = PgRow::new(self.inner.clone(), self.current_row);
            self.current_row += 1;
            Some(Ok(row))
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.current_row = (self.current_row + n).min(self.inner.db_result.num_rows());
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}

/// The type returned by various [`Connection`] methods.
/// Acts as an iterator over `T`.
#[allow(missing_debug_implementations)]
pub struct RowByRowCursor<'conn> {
    first_row: bool,
    inner: Rc<PgResultAndConn<'conn>>,
}

impl<'conn> RowByRowCursor<'conn> {
    pub(super) fn new(conn: &'conn mut PgConnection, result: PgResult) -> Self {
        RowByRowCursor {
            first_row: true,
            inner: Rc::new(PgResultAndConn {
                db_result: result,
                conn: RefCell::new(conn),
            }),
        }
    }
}

impl<'conn> Iterator for RowByRowCursor<'conn> {
    type Item = crate::QueryResult<PgRow<'conn>>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.first_row {
            let get_next_result = {
                let mut conn = self.inner.conn.borrow_mut();
                super::update_transaction_manager_status(
                    conn.connection_and_transaction_manager
                        .raw_connection
                        .get_next_result(),
                    &mut conn.connection_and_transaction_manager,
                )
            };
            match get_next_result {
                Ok(Some(res)) => {
                    // we try to reuse the existing allocation here
                    if let Some(old_res) = Rc::get_mut(&mut self.db_result) {
                        *old_res = res;
                    } else {
                        self.db_result = Rc::new(res);
                    }
                }
                Ok(None) => {
                    return None;
                }
                Err(e) => return Some(Err(e)),
            }
        }
        // This contains either 1 (for a row containing data) or 0 (for the last one) rows
        if self.inner.db_result.num_rows() > 0 {
            debug_assert_eq!(self.inner.db_result.num_rows(), 1);
            self.first_row = false;
            Some(Ok(PgRow::new(self.inner.clone(), 0)))
        } else {
            None
        }
    }
}

impl Drop for RowByRowCursor<'_> {
    fn drop(&mut self) {
        if let Ok(mut conn) = self.conn.inner.try_borrow_mut() {
            loop {
                let res = super::update_transaction_manager_status(
                    conn.connection_and_transaction_manager
                        .raw_connection
                        .get_next_result(),
                    &mut conn.connection_and_transaction_manager,
                );
                if matches!(res, Err(_) | Ok(None)) {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::DefaultLoadingMode;
    use crate::pg::PgRowByRowLoadingMode;

    #[test]
    fn fun_with_row_iters() {
        crate::table! {
            #[allow(unused_parens)]
            users(id) {
                id -> Integer,
                name -> Text,
            }
        }

        use crate::connection::LoadConnection;
        use crate::deserialize::{FromSql, FromSqlRow};
        use crate::pg::Pg;
        use crate::prelude::*;
        use crate::row::{Field, Row};
        use crate::sql_types;

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(vec![
                (users::id.eq(1), users::name.eq("Sean")),
                (users::id.eq(2), users::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();

        let query = users::table.select((users::id, users::name));

        let expected = vec![(1, String::from("Sean")), (2, String::from("Tess"))];

        let row_iter = LoadConnection::<DefaultLoadingMode>::load(conn, query).unwrap();
        for (row, expected) in row_iter.zip(&expected) {
            let row = row.unwrap();

            let deserialized = <(i32, String) as FromSqlRow<
                (sql_types::Integer, sql_types::Text),
                _,
            >>::build_from_row(&row)
            .unwrap();

            assert_eq!(&deserialized, expected);
        }

        {
            let collected_rows = LoadConnection::<DefaultLoadingMode>::load(conn, query)
                .unwrap()
                .collect::<Vec<_>>();

            for (row, expected) in collected_rows.iter().zip(&expected) {
                let deserialized = row
                    .as_ref()
                    .map(|row| {
                        <(i32, String) as FromSqlRow<
                                (sql_types::Integer, sql_types::Text),
                            _,
                            >>::build_from_row(row).unwrap()
                    })
                    .unwrap();

                assert_eq!(&deserialized, expected);
            }
        }

        let mut row_iter = LoadConnection::<DefaultLoadingMode>::load(conn, query).unwrap();

        let first_row = row_iter.next().unwrap().unwrap();
        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        let second_row = row_iter.next().unwrap().unwrap();
        let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());
        let second_values = (second_fields.0.value(), second_fields.1.value());

        assert!(row_iter.next().is_none());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(second_values.0).unwrap(),
            expected[1].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(second_values.1).unwrap(),
            expected[1].1
        );

        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );
    }

    #[test]
    fn loading_modes_return_the_same_result() {
        use crate::prelude::*;

        crate::table! {
            #[allow(unused_parens)]
            users(id) {
                id -> Integer,
                name -> Text,
            }
        }

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(vec![
                (users::id.eq(1), users::name.eq("Sean")),
                (users::id.eq(2), users::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();

        let users_by_default_mode = users::table
            .select(users::name)
            .load_iter::<String, DefaultLoadingMode>(conn)
            .unwrap()
            .collect::<QueryResult<Vec<_>>>()
            .unwrap();
        let users_row_by_row = users::table
            .select(users::name)
            .load_iter::<String, PgRowByRowLoadingMode>(conn)
            .unwrap()
            .collect::<QueryResult<Vec<_>>>()
            .unwrap();
        assert_eq!(users_by_default_mode, users_row_by_row);
        assert_eq!(users_by_default_mode, vec!["Sean", "Tess"]);
    }

    #[test]
    fn fun_with_row_iters_row_by_row() {
        crate::table! {
            #[allow(unused_parens)]
            users(id) {
                id -> Integer,
                name -> Text,
            }
        }

        use crate::connection::LoadConnection;
        use crate::deserialize::{FromSql, FromSqlRow};
        use crate::pg::Pg;
        use crate::prelude::*;
        use crate::row::{Field, Row};
        use crate::sql_types;

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        )
        .execute(conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(vec![
                (users::id.eq(1), users::name.eq("Sean")),
                (users::id.eq(2), users::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();

        let query = users::table.select((users::id, users::name));

        let expected = vec![(1, String::from("Sean")), (2, String::from("Tess"))];

        let row_iter = LoadConnection::<PgRowByRowLoadingMode>::load(conn, query).unwrap();
        for (row, expected) in row_iter.zip(&expected) {
            let row = row.unwrap();

            let deserialized = <(i32, String) as FromSqlRow<
                (sql_types::Integer, sql_types::Text),
                _,
            >>::build_from_row(&row)
            .unwrap();

            assert_eq!(&deserialized, expected);
        }

        {
            let collected_rows = LoadConnection::<PgRowByRowLoadingMode>::load(conn, query)
                .unwrap()
                .collect::<Vec<_>>();

            for (row, expected) in collected_rows.iter().zip(&expected) {
                let deserialized = row
                    .as_ref()
                    .map(|row| {
                        <(i32, String) as FromSqlRow<
                                (sql_types::Integer, sql_types::Text),
                            _,
                            >>::build_from_row(row).unwrap()
                    })
                    .unwrap();

                assert_eq!(&deserialized, expected);
            }
        }

        let mut row_iter = LoadConnection::<PgRowByRowLoadingMode>::load(conn, query).unwrap();

        let first_row = row_iter.next().unwrap().unwrap();
        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        let second_row = row_iter.next().unwrap().unwrap();
        let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());
        let second_values = (second_fields.0.value(), second_fields.1.value());

        assert!(row_iter.next().is_none());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(second_values.0).unwrap(),
            expected[1].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(second_values.1).unwrap(),
            expected[1].1
        );

        let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
        let first_values = (first_fields.0.value(), first_fields.1.value());

        assert_eq!(
            <i32 as FromSql<sql_types::Integer, Pg>>::from_nullable_sql(first_values.0).unwrap(),
            expected[0].0
        );
        assert_eq!(
            <String as FromSql<sql_types::Text, Pg>>::from_nullable_sql(first_values.1).unwrap(),
            expected[0].1
        );
    }
}
