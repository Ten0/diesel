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

allow_tables_to_appear_in_same_query!(users, posts);

// Subselect via `.single_value()` in RETURNING — this works because
// `ReturningQuerySource` implements `QuerySource`.
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
        .get_result::<Option<String>>(&mut connection)
        .unwrap();
}

// Boxed expression in RETURNING — fails because the boxed expression is
// parameterized over the table (`users::table`) rather than
// `ReturningQuerySource<UpdateStmt, users::table>`.
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
