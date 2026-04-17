use std::{borrow::Cow, usize};

use tabled::{Table, builder::Builder, settings::Style};

pub trait ToTableRecord<const N: usize> {
    fn to_record(&self) -> [Cow<'_, str>; N];
}

pub trait ToTable<T: ToTableRecord<N>, const N: usize> {
    fn table_head<IH: IntoIterator<Item = H>, H: Into<String>>(self, header: IH) -> Table;
}

impl<'a, T: ToTableRecord<N>, const N: usize> ToTable<T, N> for std::slice::Iter<'a, T> {
    fn table_head<IH: IntoIterator<Item = H>, H: Into<String>>(self, header: IH) -> Table {
        let mut table = Builder::new();

        table.push_record(header);

        for record in self.into_iter() {
            table.push_record(record.to_record());
        }
        let mut table = table.build();
        table.with(Style::markdown());
        table
    }
}
