use cfb;
use internal::expr::Expr;
use internal::stringpool::StringPool;
use internal::table::{Row, Rows, Table};
use internal::value::{Value, ValueRef};
use std::collections::{BTreeMap, HashSet};
use std::io::{self, Read, Seek, Write};
use std::rc::Rc;

// ========================================================================= //

/// A database query to delete existing rows.
pub struct Delete {
    table_name: String,
    condition: Expr,
}

impl Delete {
    /// Starts building a query that will delete rows from the specified table.
    pub fn from(table_name: &str) -> Delete {
        Delete {
            table_name: table_name.to_string(),
            condition: Expr::boolean(true),
        }
    }

    /// Adds a restriction on which rows should be deleted by the query; only
    /// rows that match the given boolean expression will be deleted.  (This
    /// method would have been called `where()`, to better match SQL, but
    /// `where` is a reserved word in Rust.)
    pub fn with(mut self, condition: Expr) -> Delete {
        self.condition = self.condition.and(condition);
        self
    }

    pub(crate) fn exec<F>(self, comp: &mut cfb::CompoundFile<F>,
                          string_pool: &mut StringPool,
                          tables: &BTreeMap<String, Rc<Table>>)
                          -> io::Result<()>
    where
        F: Read + Write + Seek,
    {
        let table = match tables.get(&self.table_name) {
            Some(table) => table,
            None => not_found!("Table {:?} does not exist", self.table_name),
        };
        // Validate the condition.
        for column_name in self.condition.column_names().into_iter() {
            if !table.has_column(column_name) {
                invalid_input!("Table {:?} has no column named {:?}",
                               self.table_name,
                               column_name);
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
            let values: Vec<Value> = value_refs
                .iter()
                .map(|value_ref| value_ref.to_value(string_pool))
                .collect();
            let row = Row::new(table.clone(), values);
            if self.condition.eval(&row).to_bool() {
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

// ========================================================================= //

/// A database query to insert new rows.
pub struct Insert {
    table_name: String,
    new_rows: Vec<Vec<Value>>,
}

impl Insert {
    /// Starts building a query that will insert rows into the specified table.
    pub fn into(table_name: &str) -> Insert {
        Insert {
            table_name: table_name.to_string(),
            new_rows: Vec::new(),
        }
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

    pub(crate) fn exec<F>(self, comp: &mut cfb::CompoundFile<F>,
                          string_pool: &mut StringPool,
                          tables: &BTreeMap<String, Rc<Table>>)
                          -> io::Result<()>
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
                invalid_input!("Table {:?} has {} columns, but a row with {} \
                                values was provided",
                               self.table_name,
                               table.columns().len(),
                               values.len());
            }
            for (column, value) in table.columns().iter().zip(values.iter()) {
                if !column.is_valid_value(value) {
                    invalid_input!("{:?} is not a valid value for column {:?}",
                                   value,
                                   column.name());
                }
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
                    invalid_data!("Malformed table {:?} contains \
                                   multiple rows with key {:?}",
                                  self.table_name,
                                  keys);
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
                already_exists!("Table {:?} already contains a row with \
                                 key {:?}",
                                self.table_name,
                                keys);
            }
            if new_keys_set.contains(&keys) {
                invalid_input!("Cannot insert multiple rows with key {:?}",
                               keys);
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
        let rows: Vec<Vec<ValueRef>> =
            rows_map.into_iter().map(|(_, row)| row).collect();
        let stream = comp.create_stream(&stream_name)?;
        table.write_rows(stream, rows)?;
        Ok(())
    }
}

// ========================================================================= //

enum Join {
    Table(String),
    Inner(Box<Select>, Box<Select>, Expr),
}

impl Join {
    fn exec<'a, F>(self, comp: &mut cfb::CompoundFile<F>,
                   string_pool: &'a StringPool,
                   tables: &BTreeMap<String, Rc<Table>>)
                   -> io::Result<Rows<'a>>
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
                let columns = table1
                    .columns()
                    .iter()
                    .map(|column| column.with_name_prefix(table1.name()))
                    .chain(table2.columns().iter().map(|column| {
                        column.with_name_prefix(table2.name())
                    }))
                    .collect();
                let table = Table::new("<join>".to_string(),
                                       columns,
                                       string_pool.long_string_refs());
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
        }
    }
}

// ========================================================================= //

/// A database query to select rows.
pub struct Select {
    from: Join,
    condition: Expr,
    column_names: Vec<String>,
}

impl Select {
    /// Starts building a query that will select rows from the specified table.
    pub fn table(table_name: &str) -> Select {
        Select {
            from: Join::Table(table_name.to_string()),
            condition: Expr::boolean(true),
            column_names: vec![],
        }
    }

    /// Performs an inner join between this and another query, producing a row
    /// for each pair of rows from the two tables that matches the expression.
    pub fn inner_join(self, rhs: Select, on: Expr) -> Select {
        Select {
            from: Join::Inner(Box::new(self), Box::new(rhs), on),
            condition: Expr::boolean(true),
            column_names: vec![],
        }
    }

    /// Transforms the selected rows to only include the specified columns, in
    /// the order given.
    pub fn columns(mut self, column_names: &[&str]) -> Select {
        self.column_names =
            column_names.iter().map(|name| name.to_string()).collect();
        self
    }

    /// Adds a restriction on which rows should be selected by the query; only
    /// rows that match the given boolean expression will be returned.  (This
    /// method would have been called `where()`, to better match SQL, but
    /// `where` is a reserved word in Rust.)
    pub fn with(mut self, condition: Expr) -> Select {
        self.condition = self.condition.and(condition);
        self
    }

    pub(crate) fn exec<'a, F>(self, comp: &mut cfb::CompoundFile<F>,
                              string_pool: &'a StringPool,
                              tables: &BTreeMap<String, Rc<Table>>)
                              -> io::Result<Rows<'a>>
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
                    invalid_input!("Table {:?} has no column named {:?}",
                                   table.name(),
                                   column_name);
                }
            }
        }
        // Validate the condition.
        for column_name in self.condition.column_names().into_iter() {
            if !table.has_column(column_name) {
                invalid_input!("Table {:?} has no column named {:?}",
                               table.name(),
                               column_name);
            }
        }
        // Filter the rows to those matching the condition.
        let condition = self.condition;
        rows.retain(|value_refs| {
                        let values: Vec<Value> = value_refs
                            .iter()
                            .map(|value_ref| value_ref.to_value(string_pool))
                            .collect();
                        let row = Row::new(table.clone(), values);
                        condition.eval(&row).to_bool()
                    });
        // Limit the table to the specified columns.
        if !column_indices.is_empty() {
            let columns = column_indices
                .iter()
                .map(|&index| table.columns()[index].clone())
                .collect();
            table = Table::new("<select>".to_string(),
                               columns,
                               table.long_string_refs());
            for value_refs in rows.iter_mut() {
                *value_refs = column_indices
                    .iter()
                    .map(|&index| value_refs[index])
                    .collect();
            }
        }
        Ok(Rows::new(string_pool, table, rows))
    }
}

// ========================================================================= //

/// A database query to update existing rows.
pub struct Update {
    table_name: String,
    updates: Vec<(String, Value)>,
    condition: Expr,
}

impl Update {
    /// Starts building a query that will update rows in the specified table.
    pub fn table(table_name: &str) -> Update {
        Update {
            table_name: table_name.to_string(),
            updates: Vec::new(),
            condition: Expr::boolean(true),
        }
    }

    /// Adds a column value to be set by the query.
    pub fn set(mut self, column_name: &str, value: Value) -> Update {
        self.updates.push((column_name.to_string(), value));
        self
    }

    /// Adds a restriction on which rows should be updated by the query; only
    /// rows that match the given boolean expression will be updated.  (This
    /// method would have been called `where()`, to better match SQL, but
    /// `where` is a reserved word in Rust.)
    pub fn with(mut self, condition: Expr) -> Update {
        self.condition = self.condition.and(condition);
        self
    }

    pub(crate) fn exec<F>(self, comp: &mut cfb::CompoundFile<F>,
                          string_pool: &mut StringPool,
                          tables: &BTreeMap<String, Rc<Table>>)
                          -> io::Result<()>
    where
        F: Read + Write + Seek,
    {
        let table = match tables.get(&self.table_name) {
            Some(table) => table,
            None => not_found!("Table {:?} does not exist", self.table_name),
        };
        // Validate the updates.
        for &(ref column_name, ref value) in self.updates.iter() {
            if !table.has_column(column_name.as_str()) {
                invalid_input!("Table {:?} has no column named {:?}",
                               self.table_name,
                               column_name);
            }
            let column = table.get_column(column_name).unwrap();
            if !column.is_valid_value(value) {
                invalid_input!("{:?} is not a valid value for column {:?}",
                               value,
                               column_name);
            }
        }
        // Validate the condition.
        for column_name in self.condition.column_names().into_iter() {
            if !table.has_column(column_name) {
                invalid_input!("Table {:?} has no column named {:?}",
                               self.table_name,
                               column_name);
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
            let values: Vec<Value> = value_refs
                .iter()
                .map(|value_ref| value_ref.to_value(string_pool))
                .collect();
            let row = Row::new(table.clone(), values);
            if self.condition.eval(&row).to_bool() {
                for &(ref column_name, ref value) in self.updates.iter() {
                    let index = row.index_for_column_name(column_name)
                        .unwrap();
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

// ========================================================================= //
