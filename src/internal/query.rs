use crate::internal::expr::Expr;
use crate::internal::stringpool::StringPool;
use crate::internal::table::{Row, Rows, Table};
use crate::internal::value::{Value, ValueRef};
use cfb;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io::{self, Read, Seek, Write};
use std::rc::Rc;

// ========================================================================= //

/// A database query to delete existing rows.
pub struct Delete {
    table_name: String,
    condition: Option<Expr>,
}

impl Delete {
    /// Starts building a query that will delete rows from the specified table.
    pub fn from<S: Into<String>>(table_name: S) -> Delete {
        Delete { table_name: table_name.into(), condition: None }
    }

    /// Adds a restriction on which rows should be deleted by the query; only
    /// rows that match the given boolean expression will be deleted.  (This
    /// method would have been called `where()`, to better match SQL, but
    /// `where` is a reserved word in Rust.)
    pub fn with(mut self, condition: Expr) -> Delete {
        self.condition = if let Some(expr) = self.condition {
            Some(expr.and(condition))
        } else {
            Some(condition)
        };
        self
    }

    pub(crate) fn exec<F>(
        self,
        comp: &mut cfb::CompoundFile<F>,
        string_pool: &mut StringPool,
        tables: &BTreeMap<String, Rc<Table>>,
    ) -> io::Result<()>
    where
        F: Read + Write + Seek,
    {
        let table = match tables.get(&self.table_name) {
            Some(table) => table,
            None => not_found!("Table {:?} does not exist", self.table_name),
        };
        // Validate the condition.
        if let Some(ref expr) = self.condition {
            for column_name in expr.column_names().into_iter() {
                if !table.has_column(column_name) {
                    invalid_input!(
                        "Table {:?} has no column named {:?}",
                        self.table_name,
                        column_name
                    );
                }
            }
        }
        // Read in the rows from the table.
        let stream_name = table.stream_name();
        let mut rows = if comp.exists(&stream_name) {
            let stream = comp.open_stream(&stream_name)?;
            table.read_rows(stream)?
        } else {
            Vec::new()
        };
        // Delete rows from the table.
        rows.retain(|value_refs| {
            let should_delete = match self.condition {
                Some(ref expr) => {
                    let values: Vec<Value> = value_refs
                        .iter()
                        .map(|value_ref| value_ref.to_value(string_pool))
                        .collect();
                    let row = Row::new(table.clone(), values);
                    expr.eval(&row).to_bool()
                }
                None => true,
            };
            // TODO: Handle deleting rows referred to by foreign keys.
            if should_delete {
                for value_ref in value_refs.iter() {
                    value_ref.remove(string_pool);
                }
                false
            } else {
                true
            }
        });
        // Write the table back out to the file.
        let stream = comp.create_stream(&stream_name)?;
        table.write_rows(stream, rows)?;
        Ok(())
    }
}

impl fmt::Display for Delete {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("DELETE FROM ")?;
        formatter.write_str(&self.table_name)?;
        if let Some(ref expr) = self.condition {
            formatter.write_str(" WHERE ")?;
            expr.fmt(formatter)?;
        }
        Ok(())
    }
}

// ========================================================================= //

/// A database query to insert new rows.
pub struct Insert {
    table_name: String,
    new_rows: Vec<Vec<Value>>,
}

impl Insert {
    /// Starts building a query that will insert rows into the specified table.
    pub fn into<S: Into<String>>(table_name: S) -> Insert {
        Insert { table_name: table_name.into(), new_rows: Vec::new() }
    }

    /// Adds a new row to be inserted into the table.
    pub fn row(mut self, values: Vec<Value>) -> Insert {
        self.new_rows.push(values);
        self
    }

    /// Adds multiple new rows to be inserted into the table.
    pub fn rows(mut self, mut rows: Vec<Vec<Value>>) -> Insert {
        self.new_rows.append(&mut rows);
        self
    }

    pub(crate) fn exec<F>(
        self,
        comp: &mut cfb::CompoundFile<F>,
        string_pool: &mut StringPool,
        tables: &BTreeMap<String, Rc<Table>>,
    ) -> io::Result<()>
    where
        F: Read + Write + Seek,
    {
        let table = match tables.get(&self.table_name) {
            Some(table) => table,
            None => not_found!("Table {:?} does not exist", self.table_name),
        };
        // Validate the new rows.
        for values in self.new_rows.iter() {
            if values.len() != table.columns().len() {
                invalid_input!(
                    "Table {:?} has {} columns, but a row with {} values was \
                     provided",
                    self.table_name,
                    table.columns().len(),
                    values.len()
                );
            }
            for (column, value) in table.columns().iter().zip(values.iter()) {
                if !column.is_valid_value(value) {
                    invalid_input!(
                        "{} is not a valid value for column {:?}",
                        value,
                        column.name()
                    );
                }
                // TODO: Validate foreign keys.
            }
        }
        // Read in the rows from the table.
        let stream_name = table.stream_name();
        let key_indices = table.primary_key_indices();
        let mut rows_map = BTreeMap::<Vec<Value>, Vec<ValueRef>>::new();
        if comp.exists(&stream_name) {
            let stream = comp.open_stream(&stream_name)?;
            for row in table.read_rows(stream)?.into_iter() {
                let keys: Vec<Value> = key_indices
                    .iter()
                    .map(|&index| row[index].to_value(string_pool))
                    .collect();
                if rows_map.contains_key(&keys) {
                    invalid_data!(
                        "Malformed table {:?} contains multiple rows with \
                         key {:?}",
                        self.table_name,
                        keys
                    );
                }
                rows_map.insert(keys, row);
            }
        }
        // Check if any of the new rows already exist in the table (or conflict
        // with each other).
        let mut new_keys_set = HashSet::<Vec<Value>>::new();
        for values in self.new_rows.iter() {
            let keys: Vec<Value> = key_indices
                .iter()
                .map(|&index| values[index].clone())
                .collect();
            if rows_map.contains_key(&keys) {
                already_exists!(
                    "Table {:?} already contains a row with key {:?}",
                    self.table_name,
                    keys
                );
            }
            if new_keys_set.contains(&keys) {
                invalid_input!(
                    "Cannot insert multiple rows with key {:?}",
                    keys
                );
            }
            new_keys_set.insert(keys);
        }
        // Insert the new rows into the table.
        for values in self.new_rows.into_iter() {
            let keys: Vec<Value> = key_indices
                .iter()
                .map(|&index| values[index].clone())
                .collect();
            let row: Vec<ValueRef> = values
                .into_iter()
                .map(|value| ValueRef::create(value, string_pool))
                .collect();
            rows_map.insert(keys, row);
        }
        // Write the table back out to the file.
        let rows: Vec<Vec<ValueRef>> = rows_map.into_values().collect();
        let stream = comp.create_stream(&stream_name)?;
        table.write_rows(stream, rows)?;
        Ok(())
    }
}

impl fmt::Display for Insert {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("INSERT INTO ")?;
        formatter.write_str(&self.table_name)?;
        if !self.new_rows.is_empty() {
            formatter.write_str(" VALUES ")?;
            let mut outer_comma = false;
            for new_row in self.new_rows.iter() {
                if outer_comma {
                    formatter.write_str(", ")?;
                } else {
                    outer_comma = true;
                }
                formatter.write_str("(")?;
                let mut inner_comma = false;
                for value in new_row.iter() {
                    if inner_comma {
                        formatter.write_str(", ")?;
                    } else {
                        inner_comma = true;
                    }
                    value.fmt(formatter)?;
                }
                formatter.write_str(")")?;
            }
        }
        Ok(())
    }
}

// ========================================================================= //

enum Join {
    Table(String),
    Inner(Box<Select>, Box<Select>, Expr),
    Left(Box<Select>, Box<Select>, Expr),
}

impl Join {
    fn exec<'a, F>(
        self,
        comp: &mut cfb::CompoundFile<F>,
        string_pool: &'a StringPool,
        tables: &BTreeMap<String, Rc<Table>>,
    ) -> io::Result<Rows<'a>>
    where
        F: Read + Seek,
    {
        match self {
            Join::Table(table_name) => {
                let table = match tables.get(&table_name) {
                    Some(table) => table,
                    None => {
                        not_found!("Table {:?} does not exist", table_name)
                    }
                };
                let stream_name = table.stream_name();
                let rows = if comp.exists(&stream_name) {
                    let stream = comp.open_stream(&stream_name)?;
                    table.read_rows(stream)?
                } else {
                    Vec::new()
                };
                Ok(Rows::new(string_pool, table.clone(), rows))
            }
            Join::Inner(select1, select2, condition) => {
                let (table1, rows1) = select1
                    .exec(comp, string_pool, tables)?
                    .into_table_and_values();
                let (table2, rows2) = select2
                    .exec(comp, string_pool, tables)?
                    .into_table_and_values();
                let columns =
                    table1
                        .columns()
                        .iter()
                        .map(|column| column.with_name_prefix(table1.name()))
                        .chain(table2.columns().iter().map(|column| {
                            column.with_name_prefix(table2.name())
                        }))
                        .collect();
                let table = Table::new(
                    String::new(),
                    columns,
                    string_pool.long_string_refs(),
                );
                let mut rows = Vec::<Vec<ValueRef>>::new();
                for value_refs1 in rows1.iter() {
                    for value_refs2 in rows2.iter() {
                        let value_refs: Vec<ValueRef> = value_refs1
                            .iter()
                            .chain(value_refs2.iter())
                            .cloned()
                            .collect();
                        let values: Vec<Value> = value_refs
                            .iter()
                            .map(|value_ref| value_ref.to_value(string_pool))
                            .collect();
                        let row = Row::new(table.clone(), values);
                        if condition.eval(&row).to_bool() {
                            rows.push(value_refs);
                        }
                    }
                }
                Ok(Rows::new(string_pool, table, rows))
            }
            Join::Left(select1, select2, condition) => {
                let (table1, rows1) = select1
                    .exec(comp, string_pool, tables)?
                    .into_table_and_values();
                let (table2, rows2) = select2
                    .exec(comp, string_pool, tables)?
                    .into_table_and_values();
                let columns = table1
                    .columns()
                    .iter()
                    .map(|column| column.with_name_prefix(table1.name()))
                    .chain(table2.columns().iter().map(|column| {
                        column.with_name_prefix(table2.name()).but_nullable()
                    }))
                    .collect();
                let table = Table::new(
                    String::new(),
                    columns,
                    string_pool.long_string_refs(),
                );
                let mut rows = Vec::<Vec<ValueRef>>::new();
                for value_refs1 in rows1.iter() {
                    let mut found_any = false;
                    for value_refs2 in rows2.iter() {
                        let value_refs: Vec<ValueRef> = value_refs1
                            .iter()
                            .chain(value_refs2.iter())
                            .cloned()
                            .collect();
                        let values: Vec<Value> = value_refs
                            .iter()
                            .map(|value_ref| value_ref.to_value(string_pool))
                            .collect();
                        let row = Row::new(table.clone(), values);
                        if condition.eval(&row).to_bool() {
                            rows.push(value_refs);
                            found_any = true;
                        }
                    }
                    if !found_any {
                        let value_refs: Vec<ValueRef> = value_refs1
                            .iter()
                            .cloned()
                            .chain(
                                table2
                                    .columns()
                                    .iter()
                                    .map(|_| ValueRef::Null),
                            )
                            .collect();
                        rows.push(value_refs);
                    }
                }
                Ok(Rows::new(string_pool, table, rows))
            }
        }
    }
}

impl fmt::Display for Join {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Join::Table(ref table_name) => table_name.fmt(formatter),
            Join::Inner(ref lhs, ref rhs, ref on) => {
                lhs.format_for_join(formatter)?;
                formatter.write_str(" INNER JOIN ")?;
                rhs.format_for_join(formatter)?;
                formatter.write_str(" ON ")?;
                on.fmt(formatter)?;
                Ok(())
            }
            Join::Left(ref lhs, ref rhs, ref on) => {
                lhs.format_for_join(formatter)?;
                formatter.write_str(" LEFT JOIN ")?;
                rhs.format_for_join(formatter)?;
                formatter.write_str(" ON ")?;
                on.fmt(formatter)?;
                Ok(())
            }
        }
    }
}

// ========================================================================= //

/// A database query to select rows.
pub struct Select {
    from: Join,
    column_names: Vec<String>,
    condition: Option<Expr>,
}

impl Select {
    /// Starts building a query that will select rows from the specified table.
    pub fn table<S: Into<String>>(table_name: S) -> Select {
        Select {
            from: Join::Table(table_name.into()),
            column_names: vec![],
            condition: None,
        }
    }

    /// Performs an inner join between this and another query, producing a row
    /// for each pair of rows from the two tables that matches the expression.
    pub fn inner_join(self, rhs: Select, on: Expr) -> Select {
        Select {
            from: Join::Inner(Box::new(self), Box::new(rhs), on),
            column_names: vec![],
            condition: None,
        }
    }

    /// Performs a left join between this and another query.
    pub fn left_join(self, rhs: Select, on: Expr) -> Select {
        Select {
            from: Join::Left(Box::new(self), Box::new(rhs), on),
            column_names: vec![],
            condition: None,
        }
    }

    // TODO: pub fn right_join

    /// Transforms the selected rows to only include the specified columns, in
    /// the order given.
    pub fn columns<S>(mut self, column_names: &[S]) -> Select
    where
        S: Clone + Into<String>,
    {
        self.column_names =
            column_names.iter().cloned().map(|name| name.into()).collect();
        self
    }

    /// Adds a restriction on which rows should be selected by the query; only
    /// rows that match the given boolean expression will be returned.  (This
    /// method would have been called `where()`, to better match SQL, but
    /// `where` is a reserved word in Rust.)
    pub fn with(mut self, condition: Expr) -> Select {
        self.condition = if let Some(expr) = self.condition {
            Some(expr.and(condition))
        } else {
            Some(condition)
        };
        self
    }

    pub(crate) fn exec<'a, F>(
        self,
        comp: &mut cfb::CompoundFile<F>,
        string_pool: &'a StringPool,
        tables: &BTreeMap<String, Rc<Table>>,
    ) -> io::Result<Rows<'a>>
    where
        F: Read + Seek,
    {
        // Join the table(s) to be queried.
        let rows = self.from.exec(comp, string_pool, tables)?;
        let (mut table, mut rows) = rows.into_table_and_values();
        // Validate the selected column names.
        let mut column_indices =
            Vec::<usize>::with_capacity(self.column_names.len());
        for column_name in self.column_names.iter() {
            match table.index_for_column_name(column_name.as_str()) {
                Some(index) => column_indices.push(index),
                None => {
                    invalid_input!(
                        "Table {:?} has no column named {:?}",
                        table.name(),
                        column_name
                    );
                }
            }
        }
        // Validate the condition.
        if let Some(ref expr) = self.condition {
            for column_name in expr.column_names().into_iter() {
                if !table.has_column(column_name) {
                    invalid_input!(
                        "Table {:?} has no column named {:?}",
                        table.name(),
                        column_name
                    );
                }
            }
        }
        // Filter the rows to those matching the condition.
        if let Some(condition) = self.condition {
            rows.retain(|value_refs| {
                let values: Vec<Value> = value_refs
                    .iter()
                    .map(|value_ref| value_ref.to_value(string_pool))
                    .collect();
                let row = Row::new(table.clone(), values);
                condition.eval(&row).to_bool()
            });
        }
        // Limit the table to the specified columns.
        if !column_indices.is_empty() {
            let columns = column_indices
                .iter()
                .map(|&index| table.columns()[index].clone())
                .collect();
            table =
                Table::new(String::new(), columns, table.long_string_refs());
            for value_refs in rows.iter_mut() {
                *value_refs = column_indices
                    .iter()
                    .map(|&index| value_refs[index])
                    .collect();
            }
        }
        Ok(Rows::new(string_pool, table, rows))
    }

    fn format_for_join(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        if self.column_names.is_empty() && self.condition.is_none() {
            if let Join::Table(ref name) = self.from {
                return formatter.write_str(name.as_str());
            }
        }
        formatter.write_str("(")?;
        fmt::Display::fmt(self, formatter)?;
        formatter.write_str(")")?;
        Ok(())
    }
}

impl fmt::Display for Select {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("SELECT ")?;
        if self.column_names.is_empty() {
            formatter.write_str("*")?;
        } else {
            let mut comma = false;
            for column_name in self.column_names.iter() {
                if comma {
                    formatter.write_str(", ")?;
                } else {
                    comma = true;
                }
                formatter.write_str(column_name)?;
            }
        }
        formatter.write_str(" FROM ")?;
        self.from.fmt(formatter)?;
        if let Some(ref expr) = self.condition {
            formatter.write_str(" WHERE ")?;
            expr.fmt(formatter)?;
        }
        Ok(())
    }
}

// ========================================================================= //

/// A database query to update existing rows.
pub struct Update {
    table_name: String,
    updates: Vec<(String, Value)>,
    condition: Option<Expr>,
}

impl Update {
    /// Starts building a query that will update rows in the specified table.
    pub fn table<S: Into<String>>(table_name: S) -> Update {
        Update {
            table_name: table_name.into(),
            updates: Vec::new(),
            condition: None,
        }
    }

    /// Adds a column value to be set by the query.
    pub fn set<S: Into<String>>(
        mut self,
        column_name: S,
        value: Value,
    ) -> Update {
        self.updates.push((column_name.into(), value));
        self
    }

    /// Adds a restriction on which rows should be updated by the query; only
    /// rows that match the given boolean expression will be updated.  (This
    /// method would have been called `where()`, to better match SQL, but
    /// `where` is a reserved word in Rust.)
    pub fn with(mut self, condition: Expr) -> Update {
        self.condition = if let Some(expr) = self.condition {
            Some(expr.and(condition))
        } else {
            Some(condition)
        };
        self
    }

    pub(crate) fn exec<F>(
        self,
        comp: &mut cfb::CompoundFile<F>,
        string_pool: &mut StringPool,
        tables: &BTreeMap<String, Rc<Table>>,
    ) -> io::Result<()>
    where
        F: Read + Write + Seek,
    {
        let table = match tables.get(&self.table_name) {
            Some(table) => table,
            None => not_found!("Table {:?} does not exist", self.table_name),
        };
        // Validate the updates.
        for (column_name, value) in self.updates.iter() {
            if !table.has_column(column_name.as_str()) {
                invalid_input!(
                    "Table {:?} has no column named {:?}",
                    self.table_name,
                    column_name
                );
            }
            let column = table.get_column(column_name).unwrap();
            if !column.is_valid_value(value) {
                invalid_input!(
                    "{} is not a valid value for column {:?}",
                    value,
                    column_name
                );
            }
            // TODO: Validate foreign keys.
        }
        // Validate the condition.
        if let Some(ref expr) = self.condition {
            for column_name in expr.column_names().into_iter() {
                if !table.has_column(column_name) {
                    invalid_input!(
                        "Table {:?} has no column named {:?}",
                        self.table_name,
                        column_name
                    );
                }
            }
        }
        // Read in the rows from the table.
        let stream_name = table.stream_name();
        let mut rows = if comp.exists(&stream_name) {
            let stream = comp.open_stream(&stream_name)?;
            table.read_rows(stream)?
        } else {
            Vec::new()
        };
        // Update the rows.
        for value_refs in rows.iter_mut() {
            let should_update = match self.condition {
                Some(ref expr) => {
                    let values: Vec<Value> = value_refs
                        .iter()
                        .map(|value_ref| value_ref.to_value(string_pool))
                        .collect();
                    let row = Row::new(table.clone(), values);
                    expr.eval(&row).to_bool()
                }
                None => true,
            };
            if should_update {
                for (column_name, value) in self.updates.iter() {
                    let index =
                        table.index_for_column_name(column_name).unwrap();
                    let value_ref = &mut value_refs[index];
                    value_ref.remove(string_pool);
                    *value_ref = ValueRef::create(value.clone(), string_pool);
                }
            }
        }
        // Write the table back out to the file.
        let stream = comp.create_stream(&stream_name)?;
        table.write_rows(stream, rows)?;
        Ok(())
    }
}

impl fmt::Display for Update {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("UPDATE ")?;
        formatter.write_str(&self.table_name)?;
        formatter.write_str(" SET ")?;
        let mut comma = false;
        for (column_name, value) in self.updates.iter() {
            if comma {
                formatter.write_str(", ")?;
            } else {
                comma = true;
            }
            formatter.write_str(column_name)?;
            formatter.write_str(" = ")?;
            value.fmt(formatter)?;
        }
        if let Some(ref expr) = self.condition {
            formatter.write_str(" WHERE ")?;
            expr.fmt(formatter)?;
        }
        Ok(())
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Delete, Insert, Select, Update};
    use crate::internal::expr::Expr;
    use crate::internal::value::Value;

    #[test]
    fn display_delete() {
        let query = Delete::from("Foobar");
        assert_eq!(format!("{query}"), "DELETE FROM Foobar".to_string());

        let query = Delete::from("Foobar")
            .with(Expr::col("Foo").lt(Expr::integer(17)));
        assert_eq!(
            format!("{query}"),
            "DELETE FROM Foobar WHERE Foo < 17".to_string()
        );
    }

    #[test]
    fn display_insert() {
        let query = Insert::into("Foobar");
        assert_eq!(format!("{query}"), "INSERT INTO Foobar".to_string());

        let query =
            Insert::into("Foobar").row(vec![Value::from("Foo"), Value::Null]);
        assert_eq!(
            format!("{query}"),
            "INSERT INTO Foobar VALUES (\"Foo\", NULL)".to_string()
        );

        let query = Insert::into("Foobar")
            .row(vec![Value::Int(1), Value::Int(2)])
            .rows(vec![
                vec![Value::Int(3), Value::Int(4)],
                vec![Value::Int(5), Value::Int(6)],
            ])
            .row(vec![Value::Int(7), Value::Int(8)]);
        assert_eq!(
            format!("{query}"),
            "INSERT INTO Foobar VALUES (1, 2), (3, 4), (5, 6), (7, 8)"
                .to_string()
        );
    }

    #[test]
    fn display_select() {
        let query = Select::table("Foobar");
        assert_eq!(format!("{query}"), "SELECT * FROM Foobar".to_string());

        let query = Select::table("Foobar")
            .columns(&["Foo", "Bar"])
            .with(Expr::col("Foo").lt(Expr::integer(17)));
        assert_eq!(
            format!("{query}"),
            "SELECT Foo, Bar FROM Foobar WHERE Foo < 17".to_string()
        );

        let query = Select::table("Foobar")
            .inner_join(
                Select::table("Quux"),
                Expr::col("Foobar.Key").eq(Expr::col("Quux.Quay")),
            )
            .columns(&["Foobar.Foo", "Quux.Baz"]);
        assert_eq!(
            format!("{query}"),
            "SELECT Foobar.Foo, Quux.Baz FROM Foobar INNER JOIN \
                    Quux ON Foobar.Key = Quux.Quay"
                .to_string()
        );

        let query = Select::table("Foobar")
            .inner_join(
                Select::table("Quux")
                    .with(Expr::col("Quay").gt(Expr::integer(42))),
                Expr::col("Foobar.Key").eq(Expr::col("Quux.Quay")),
            )
            .columns(&["Foobar.Foo", "Quux.Baz"]);
        assert_eq!(
            format!("{query}"),
            "SELECT Foobar.Foo, Quux.Baz \
                    FROM Foobar \
                    INNER JOIN (SELECT * FROM Quux WHERE Quay > 42) \
                    ON Foobar.Key = Quux.Quay"
                .to_string()
        );
    }

    #[test]
    fn display_update() {
        let query = Update::table("Foobar").set("Foo", Value::Int(17));
        assert_eq!(
            format!("{query}"),
            "UPDATE Foobar SET Foo = 17".to_string()
        );

        let query = Update::table("Foobar")
            .set("Foo", Value::Int(17))
            .set("Bar", Value::Null)
            .set("Baz", Value::from("quux"))
            .with(Expr::col("Foo").lt(Expr::integer(17)));
        assert_eq!(
            format!("{query}"),
            "UPDATE Foobar SET Foo = 17, Bar = NULL, Baz = \"quux\" \
                    WHERE Foo < 17"
                .to_string()
        );
    }
}

// ========================================================================= //
