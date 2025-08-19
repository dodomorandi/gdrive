use std::fmt::{self, Display};
use std::io;
use std::io::Write;
use tabwriter::TabWriter;

pub struct Table<H: Display, V: Display, const COLUMNS: usize> {
    pub header: [H; COLUMNS],
    pub values: Vec<[V; COLUMNS]>,
}

#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub skip_header: bool,
    pub separator: String,
}

impl DisplayConfig {
    fn display_row<'a, T>(&'a self, value: &'a [T]) -> DisplayRow<'a, T>
    where
        T: Display,
    {
        DisplayRow(self, value)
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            skip_header: false,
            separator: String::from("\t"),
        }
    }
}

pub fn write<W: Write, H: Display, V: Display, const COLUMNS: usize>(
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
pub struct DisplayRow<'a, T>(&'a DisplayConfig, &'a [T]);

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
