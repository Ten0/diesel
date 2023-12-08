use super::cursor::PgResultAndConn;
use crate::backend::Backend;
use crate::pg::value::TypeOidLookup;
use crate::pg::{Pg, PgValue};
use crate::row::*;
use std::rc::Rc;

#[allow(missing_debug_implementations)]
pub struct PgRow<'conn> {
    pg_result_and_conn: Rc<PgResultAndConn<'conn>>,
    row_idx: usize,
}

impl<'conn> PgRow<'conn> {
    pub(crate) fn new(pg_result_and_conn: Rc<PgResultAndConn<'conn>>, row_idx: usize) -> Self {
        PgRow {
            pg_result_and_conn,
            row_idx,
        }
    }
}

impl RowSealed for PgRow<'_> {}

impl<'conn> Row<'conn, Pg> for PgRow<'conn> {
    type Field<'f> = PgField<'f, 'f> where 'conn: 'f, Self: 'f;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.pg_result_and_conn.db_result.column_count()
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'conn: 'b,
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(PgField {
            pg_result_and_conn: &self.pg_result_and_conn,
            row_idx: self.row_idx,
            col_idx: idx,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for PgRow<'_> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a> RowIndex<&'a str> for PgRow<'_> {
    fn idx(&self, field_name: &'a str) -> Option<usize> {
        (0..self.field_count())
            .find(|idx| self.pg_result_and_conn.db_result.column_name(*idx) == Some(field_name))
    }
}

#[allow(missing_debug_implementations)]
pub struct PgField<'a, 'conn> {
    pg_result_and_conn: &'a PgResultAndConn<'conn>,
    row_idx: usize,
    col_idx: usize,
}

impl<'a> Field<'a, Pg> for PgField<'a, '_> {
    fn field_name(&self) -> Option<&str> {
        self.pg_result_and_conn.db_result.column_name(self.col_idx)
    }

    fn value(&self) -> Option<<Pg as Backend>::RawValue<'_>> {
        let raw = self
            .pg_result_and_conn
            .db_result
            .get(self.row_idx, self.col_idx)?;

        Some(PgValue::new_internal(
            raw,
            self,
            &self.pg_result_and_conn.conn,
        ))
    }
}

impl TypeOidLookup for PgField<'_, '_> {
    fn lookup(&self) -> std::num::NonZeroU32 {
        self.pg_result_and_conn.db_result.column_type(self.col_idx)
    }
}
