extern crate diesel;

use diesel::pg::Pg;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

// Boxed expression in RETURNING — fails because the boxed expression is
// parameterized over the table (`users::table`) rather than
// `ReturningQuerySource<UpdateStmt, users::table>`.
fn main() {
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
