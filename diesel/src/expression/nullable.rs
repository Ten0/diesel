use crate::backend::DieselReserveSpecialization;
use crate::expression::*;
use crate::query_builder::*;
use crate::query_source::joins::ToInnerJoin;
use crate::result::QueryResult;
use crate::sql_types::{DieselNumericOps, IntoNullable};

#[doc(hidden)] // This is used by the `table!` macro internally
#[derive(Debug, Copy, Clone, DieselNumericOps, ValidGrouping)]
pub struct Nullable<T>(pub(crate) T);

impl<T> Nullable<T> {
    pub(crate) fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T>
where
    T: Expression,
    T::SqlType: IntoNullable,
    <T::SqlType as IntoNullable>::Nullable: TypedExpressionType,
{
    type SqlType = <T::SqlType as IntoNullable>::Nullable;
}

impl<T, DB> QueryFragment<DB> for Nullable<T>
where
    DB: Backend + DieselReserveSpecialization,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<T, QS> AppearsOnTable<QS> for Nullable<T>
where
    T: AppearsOnTable<QS>,
    Nullable<T>: Expression,
{
}

impl<T: QueryId> QueryId for Nullable<T> {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, QS> SelectableExpression<QS> for Nullable<T>
where
    Self: AppearsOnTable<QS>,
    QS: ToInnerJoin,
    T: SelectableExpression<QS::InnerJoin>,
{
}

impl<T> SelectableExpression<NoFromClause> for Nullable<T> where Self: AppearsOnTable<NoFromClause> {}

/// The point of this function is to make sure that users don't accidentally
/// write queries where Diesel can't differentiate between failed left join and
/// succeeded left join with all fields inside being null
///
/// That would lead to `Some(Struct { None, None, ... })`
/// where in fact the left join failed and the user would expect just `None`,
/// but Diesel can't differentiate between the two cases because it won't be
/// catching an `UnexpectedNullError` in the `FromStaticSqlRow` implementation
/// for `Option<T>`, because they will all be caught by the earlier field.
pub(crate) const fn from_static_sql_row_for_option_check_not_all_nullable<
    OriginalTupleFields,
    NullableTupleFields,
>() {
    // If the types are equal, this means that all the fields in the tuple are
    // nullable. this is not OK, because in that case Option<T> would not
    // make sense because as the inner fields would always succed to deserialize
    // (as None), and we would never be able to catch `UnexpectedNullError`
    // and generate `None` on the outer `Option`.

    // Another way to express this would be: if all the
    // fields in the tuple are nullable, then there is no way to
    // differentiate in the returned row between a failed left join and a
    // successful left join with all fields null.

    // Here we would want to compare the types by their `TypeId` (returning an
    // error if they ARE equal), but that is not possible in const context
    // yet. This branch will be waiting for stabilization of:
    // - TypeId in const: https://github.com/rust-lang/rust/issues/77125
    // - TypeId::eq in const: https://github.com/rust-lang/rust/blob/09593059867b066d23776e00a2ce31c370ec9277/tests/ui/consts/const_cmp_type_id.rs

    // An alternate implementation could be to recursively go through all the
    // fields of the tuple with AllAreNullable to give an error if they all are,
    // but I fear that this might get too expensive in terms of compilation
    // time, plus the implementation would be more complex, if even possible at
    // all...
}
