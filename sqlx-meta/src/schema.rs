pub struct Metadata<'s, const C: usize> {
    pub table_name: &'s str,
    pub id_column: &'s str,
    pub columns: [&'s str; C],
}
