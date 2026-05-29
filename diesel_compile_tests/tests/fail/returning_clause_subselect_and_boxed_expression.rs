extern crate diesel;

use diesel::pg::Pg;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        title -> VarChar,
    }
}

// Subselect via `.single_value()` in RETURNING — used to work before
// `ReturningQuerySource` was introduced.
// Fails because `ValidSubselect<QS>` for `SelectStatement<FromClause<F>, ...>`
// requires `QS: QuerySource`, and `ReturningQuerySource` does not implement
// `QuerySource`.
fn subselect_in_returning() {
    use self::users::dsl::*;
    let mut connection = PgConnection::establish("").unwrap();

    let subselect = posts::table
        .select(posts::title)
        .filter(posts::user_id.eq(id))
        .single_value();

    diesel::update(users.filter(id.eq(1)))
        .set(name.eq("Updated"))
        .returning(subselect)
        //~^ ERROR: the trait bound `ReturningQuerySource<UpdateStmt, ...>: QuerySource` is not satisfied
        //~| ERROR: the trait bound `FromClause<Join<table, ..., ...>>: AsQuerySource` is not satisfied
        //~| ERROR: the trait bound `Join<table, ..., ...>: AppearsInFromClause<...>` is not satisfied
        .get_result::<Option<String>>(&mut connection)
        //~^ ERROR: the trait bound `ReturningQuerySource<UpdateStmt, ...>: QuerySource` is not satisfied
        //~| ERROR: the trait bound `FromClause<Join<table, ..., ...>>: AsQuerySource` is not satisfied
        //~| ERROR: the trait bound `Join<table, ..., ...>: AppearsInFromClause<...>` is not satisfied
        .unwrap();
}

// Boxed expression in RETURNING — used to work before
// `ReturningQuerySource` was introduced.
// Fails because the boxed expression is parameterized over the table
// (`users::table`) rather than `ReturningQuerySource<UpdateStmt, users::table>`.
fn boxed_expression_in_returning() {
    let mut connection = PgConnection::establish("").unwrap();

    let boxed_expr: Box<
        dyn BoxableExpression<users::table, Pg, SqlType = diesel::sql_types::Text>,
    > = Box::new(users::name);

    diesel::update(users::table.filter(users::id.eq(1)))
        .set(users::name.eq("Updated"))
        .returning(boxed_expr)
        //~^ ERROR: cannot select `dyn BoxableExpression<table, Pg, SqlType = Text>` from `ReturningQuerySource<UpdateStmt, table>`
        .get_result::<String>(&mut connection)
        //~^ ERROR: cannot select `dyn BoxableExpression<table, Pg, SqlType = Text>` from `ReturningQuerySource<UpdateStmt, table>`
        .unwrap();
}

fn main() {
    subselect_in_returning();
    boxed_expression_in_returning();
}
