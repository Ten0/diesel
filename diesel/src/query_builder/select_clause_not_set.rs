use crate::backend::Backend;
use crate::expression::is_aggregate;
use crate::expression::{expression_types::NotSelectable, Expression, ValidGrouping};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
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
    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        // We should enforce setting a select clause if not provided
        // but that would require updating a lot of trait bounds, so for this
        // prototype we'll just make it a valid query and prevent from loading it into
        // anything
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
