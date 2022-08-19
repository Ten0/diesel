use crate::backend::Backend;
use crate::expression::is_aggregate;
use crate::expression::{expression_types::NotSelectable, Expression, ValidGrouping};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::aliasing::{Alias, FieldAliasMapper};
use crate::{AppearsOnTable, QueryResult, SelectableExpression};

#[doc(hidden)]
#[derive(Debug, Copy, Clone)]
pub struct SelectClauseNotSet;

impl Expression for SelectClauseNotSet {
    type SqlType = NotSelectable;
}

impl<DB> QueryFragment<DB> for SelectClauseNotSet
where
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        // This is an acceptable serialization because it will not allow selecting into
        // anything or storing into anything (because no SQL type matches) but will still
        // be correct for `dsl::exists(table.filter(something))`, as that will build to
        // `EXISTS (SELECT 1 FROM table WHERE something)`
        // In addition, the fact this is a valid expression enables keeping the usual trait bounds
        // everywhere
        pass.push_sql("1");
        Ok(())
    }
}

impl QueryId for SelectClauseNotSet {
    type QueryId = SelectClauseNotSet;

    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<QS> SelectableExpression<QS> for SelectClauseNotSet where SelectClauseNotSet: AppearsOnTable<QS>
{}

impl<QS> AppearsOnTable<QS> for SelectClauseNotSet where SelectClauseNotSet: Expression {}

impl<GB> ValidGrouping<GB> for SelectClauseNotSet {
    type IsAggregate = is_aggregate::Never;
}

impl<S> FieldAliasMapper<S> for SelectClauseNotSet {
    type Out = SelectClauseNotSet;

    fn map(self, _alias: &Alias<S>) -> Self::Out {
        self
    }
}
