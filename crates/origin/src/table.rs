use std::{iter::repeat, usize};

use tabled::{Table, builder::Builder, settings::Style};
use unicode_width::{UnicodeWidthChar as _, UnicodeWidthStr as _};

use crate::entity::ToTableRecord;

/// Return head len sub-string of s (unicode-width)
pub fn head(s: impl AsRef<str>, len: usize) -> String {
    if s.as_ref().width_cjk() <= len {
        return s.as_ref().chars().chain(repeat(' ')).take(len).collect();
    }
    let mut n = 0;
    let mut cur = 0;
    for c in s.as_ref().chars() {
        cur += c.width_cjk().unwrap_or_default();
        if cur > len {
            break;
        }
        n += 1;
    }
    s.as_ref().chars().chain(repeat(' ')).take(n).collect()
}

pub trait IntoTable<T: ToTableRecord<N>, const N: usize> {
    fn table_head<IH: IntoIterator<Item = H>, H: Into<String>>(self, header: IH) -> Table;
}

impl<I: IntoIterator<Item = T>, T: ToTableRecord<N>, const N: usize> IntoTable<T, N> for I {
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
