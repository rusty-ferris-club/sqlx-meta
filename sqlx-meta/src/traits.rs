use sqlx::database::HasArguments;
use sqlx::query::QueryAs;
use sqlx::{Database, Encode, Executor, FromRow, IntoArguments, Type};

pub trait Schema {
    type Id: Send;

    fn table_name() -> &'static str;

    /// Returns the id of the current instance.
    fn id(&self) -> Self::Id;

    /// Returns the column name of the primary key.
    fn id_column() -> &'static str;

    /// Returns an array of column names.
    fn columns() -> &'static [&'static str];
}

pub trait Binds<'e, E>
where
    Self: 'e + Sized + Send + Unpin + for<'r> FromRow<'r, <E::Database as Database>::Row> + Schema,
    <Self as Schema>::Id:
        Encode<'e, <E as Executor<'e>>::Database> + Type<<E as Executor<'e>>::Database>,
    E: Executor<'e> + 'e,
    <E::Database as HasArguments<'e>>::Arguments: IntoArguments<'e, <E as Executor<'e>>::Database>,
{
    fn insert_binds(
        &'e self,
        query: QueryAs<'e, E::Database, Self, <E::Database as HasArguments<'e>>::Arguments>,
    ) -> QueryAs<'e, E::Database, Self, <E::Database as HasArguments<'e>>::Arguments>;

    fn update_binds(
        &'e self,
        query: QueryAs<'e, E::Database, Self, <E::Database as HasArguments<'e>>::Arguments>,
    ) -> QueryAs<'e, E::Database, Self, <E::Database as HasArguments<'e>>::Arguments>;
}
