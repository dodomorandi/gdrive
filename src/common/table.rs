use std::{
    fmt::{self, Display},
    io::{self, Write},
};

use tabwriter::TabWriter;

pub(crate) struct Table<H: Display, V: Display, const COLUMNS: usize> {
    pub(crate) header: [H; COLUMNS],
    pub(crate) values: Vec<[V; COLUMNS]>,
}

#[derive(Debug, Clone)]
pub(crate) struct DisplayConfig<'a> {
    pub(crate) skip_header: bool,
    pub(crate) separator: &'a str,
}

impl DisplayConfig<'_> {
    fn display_row<'a, T>(&'a self, value: &'a [T]) -> DisplayRow<'a, T>
    where
        T: Display,
    {
        DisplayRow(self, value)
    }
}

impl Default for DisplayConfig<'static> {
    fn default() -> Self {
        Self {
            skip_header: false,
            separator: "\t",
        }
    }
}

pub(crate) fn write<W: Write, H: Display, V: Display, const COLUMNS: usize>(
    writer: W,
    table: Table<H, V, COLUMNS>,
    config: &DisplayConfig,
) -> Result<(), io::Error> {
    let mut tw = TabWriter::new(writer).padding(3);

    if !config.skip_header {
        writeln!(&mut tw, "{}", config.display_row(&table.header))?;
    }

    for value in table.values {
        writeln!(&mut tw, "{}", config.display_row(&value))?;
    }

    tw.flush()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DisplayRow<'a, T>(&'a DisplayConfig<'a>, &'a [T]);

impl<T> Display for DisplayRow<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = self.1.iter();
        if let Some(column) = columns.next() {
            write!(f, "{column}")?;
            for column in columns {
                write!(f, "{}{column}", self.0.separator)?;
            }
        }
        Ok(())
    }
}
