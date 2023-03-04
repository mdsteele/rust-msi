extern crate clap;
extern crate msi;
extern crate pest;
#[macro_use]
extern crate pest_derive;

use clap::{App, Arg};
use msi::{Delete, Expr, Insert, Select, Update, Value};
use pest::Parser;

#[derive(Parser)]
#[grammar = "../examples/msiquery.pest"]
struct QueryParser;

type Package = msi::Package<std::fs::File>;
type Pair<'a> = pest::iterators::Pair<'a, Rule>;
type Pairs<'a> = pest::iterators::Pairs<'a, Rule>;

fn main() {
    let matches = App::new("msiquery")
        .version("0.1")
        .author("Matthew D. Steele <mdsteele@alum.mit.edu>")
        .about("Performs SQL queries on MSI files")
        .arg(Arg::with_name("path").required(true))
        .arg(Arg::with_name("query").required(true))
        .get_matches();
    let path = matches.value_of("path").unwrap();
    let query = matches.value_of("query").unwrap();
    let mut package = msi::open_rw(path).expect("open package");
    for pair in QueryParser::parse(Rule::QueryList, query).expect("parse") {
        process_query(pair, &mut package);
    }
}

fn process_query(pair: Pair, package: &mut Package) {
    match pair.as_rule() {
        Rule::QueryDelete => process_delete_query(pair, package),
        Rule::QueryInsert => process_insert_query(pair, package),
        Rule::QuerySelect => process_select_query(pair, package),
        Rule::QueryUpdate => process_update_query(pair, package),
        Rule::EOI => {}
        _ => unreachable!(),
    }
}

fn process_delete_query(pair: Pair, package: &mut Package) {
    let query = parse_delete_query(pair);
    package.delete_rows(query).unwrap();
}

fn process_insert_query(pair: Pair, package: &mut Package) {
    let query = parse_insert_query(pair);
    package.insert_rows(query).unwrap();
}

fn process_select_query(pair: Pair, package: &mut Package) {
    let query = parse_select_query(pair);
    println!("{query}");
    let rows = package.select_rows(query).unwrap();
    let columns = rows.columns().to_vec();
    let mut col_widths: Vec<usize> =
        columns.iter().map(|column| column.name().len()).collect();
    let row_strings: Vec<Vec<String>> = rows
        .map(|row| {
            let mut strings = Vec::with_capacity(row.len());
            for index in 0..row.len() {
                let string = row[index].to_string();
                col_widths[index] = col_widths[index].max(string.len());
                strings.push(string);
            }
            strings
        })
        .collect();
    {
        let mut line = String::new();
        for (index, column) in columns.iter().enumerate() {
            let string =
                pad(column.name().to_string(), ' ', col_widths[index]);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
    {
        let mut line = String::new();
        for &width in col_widths.iter() {
            let string = pad(String::new(), '-', width);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
    for row in row_strings.into_iter() {
        let mut line = String::new();
        for (index, value) in row.into_iter().enumerate() {
            let string = pad(value, ' ', col_widths[index]);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
}

fn process_update_query(pair: Pair, package: &mut Package) {
    let query = parse_update_query(pair);
    package.update_rows(query).unwrap();
}

fn parse_delete_query(pair: Pair) -> Delete {
    assert_eq!(pair.as_rule(), Rule::QueryDelete);
    let mut pairs = pair.into_inner();
    expect_token(Rule::KwDelete, &mut pairs);
    expect_token(Rule::KwFrom, &mut pairs);
    let table_name = expect_token(Rule::Ident, &mut pairs).as_str();
    let mut query = Delete::from(table_name);
    if optional_token(Rule::KwWhere, &mut pairs).is_some() {
        let expr = parse_expr(pairs.next().unwrap());
        query = query.with(expr);
    }
    expect_done(&mut pairs);
    query
}

fn parse_insert_query(pair: Pair) -> Insert {
    assert_eq!(pair.as_rule(), Rule::QueryInsert);
    let mut pairs = pair.into_inner();
    expect_token(Rule::KwInsert, &mut pairs);
    expect_token(Rule::KwInto, &mut pairs);
    let table_name = expect_token(Rule::Ident, &mut pairs).as_str();
    let mut query = Insert::into(table_name);
    if optional_token(Rule::KwValues, &mut pairs).is_some() {
        let rows = parse_row_list(expect_token(Rule::RowList, &mut pairs));
        query = query.rows(rows);
    }
    expect_done(&mut pairs);
    query
}

fn parse_select_query(pair: Pair) -> Select {
    assert_eq!(pair.as_rule(), Rule::QuerySelect);
    let mut pairs = pair.into_inner();
    expect_token(Rule::KwSelect, &mut pairs);
    let column_names = parse_column_list(pairs.next().unwrap());
    expect_token(Rule::KwFrom, &mut pairs);
    let mut query = parse_table(pairs.next().unwrap());
    if !column_names.is_empty() {
        query = query.columns(&column_names);
    }
    if optional_token(Rule::KwWhere, &mut pairs).is_some() {
        let expr = parse_expr(pairs.next().unwrap());
        query = query.with(expr);
    }
    expect_done(&mut pairs);
    query
}

fn parse_update_query(pair: Pair) -> Update {
    assert_eq!(pair.as_rule(), Rule::QueryUpdate);
    let mut pairs = pair.into_inner();
    expect_token(Rule::KwUpdate, &mut pairs);
    let table_name = expect_token(Rule::Ident, &mut pairs).as_str();
    let mut query = Update::table(table_name);
    expect_token(Rule::KwSet, &mut pairs);
    let assignments = expect_token(Rule::AssignmentList, &mut pairs);
    for (name, value) in parse_assignment_list(assignments) {
        query = query.set(name, value);
    }
    if optional_token(Rule::KwWhere, &mut pairs).is_some() {
        let expr = parse_expr(pairs.next().unwrap());
        query = query.with(expr);
    }
    expect_done(&mut pairs);
    query
}

fn parse_assignment_list(pair: Pair) -> Vec<(String, Value)> {
    assert_eq!(pair.as_rule(), Rule::AssignmentList);
    let mut pairs = pair.into_inner();
    let mut assignments = Vec::<(String, Value)>::new();
    assignments.push(parse_assignment(pairs.next().unwrap()));
    while optional_token(Rule::OpComma, &mut pairs).is_some() {
        assignments.push(parse_assignment(pairs.next().unwrap()));
    }
    assignments
}

fn parse_assignment(pair: Pair) -> (String, Value) {
    assert_eq!(pair.as_rule(), Rule::Assignment);
    let mut pairs = pair.into_inner();
    let name = expect_token(Rule::Ident, &mut pairs).as_str().to_string();
    expect_token(Rule::OpEq, &mut pairs);
    let value = parse_value(pairs.next().unwrap());
    (name, value)
}

fn parse_column_list(pair: Pair) -> Vec<String> {
    match pair.as_rule() {
        Rule::CompoundIdentList => parse_compound_ident_list(pair),
        Rule::OpStar => vec![],
        _ => unreachable!(),
    }
}

fn parse_compound_ident_list(pair: Pair) -> Vec<String> {
    assert_eq!(pair.as_rule(), Rule::CompoundIdentList);
    let mut pairs = pair.into_inner();
    let mut idents = Vec::<String>::new();
    idents.push(
        expect_token(Rule::CompoundIdent, &mut pairs).as_str().to_string(),
    );
    while optional_token(Rule::OpComma, &mut pairs).is_some() {
        idents.push(
            expect_token(Rule::CompoundIdent, &mut pairs).as_str().to_string(),
        );
    }
    idents
}

fn parse_row_list(pair: Pair) -> Vec<Vec<Value>> {
    assert_eq!(pair.as_rule(), Rule::RowList);
    let mut pairs = pair.into_inner();
    let mut rows = Vec::<Vec<Value>>::new();
    rows.push(parse_row(pairs.next().unwrap()));
    while optional_token(Rule::OpComma, &mut pairs).is_some() {
        rows.push(parse_row(pairs.next().unwrap()));
    }
    rows
}

fn parse_row(pair: Pair) -> Vec<Value> {
    assert_eq!(pair.as_rule(), Rule::Row);
    let mut pairs = pair.into_inner();
    let mut row = Vec::<Value>::new();
    row.push(parse_value(pairs.next().unwrap()));
    while optional_token(Rule::OpComma, &mut pairs).is_some() {
        row.push(parse_value(pairs.next().unwrap()));
    }
    row
}

fn parse_table(pair: Pair) -> Select {
    match pair.as_rule() {
        Rule::Ident => Select::table(pair.as_str()),
        Rule::QuerySelect => parse_select_query(pair),
        Rule::TableJoin => parse_table_join(pair),
        _ => unreachable!(),
    }
}

fn parse_table_join(pair: Pair) -> Select {
    assert_eq!(pair.as_rule(), Rule::TableJoin);
    let mut pairs = pair.into_inner();
    let table1 = parse_table(pairs.next().unwrap());
    let kind = pairs.next().unwrap();
    expect_token(Rule::KwJoin, &mut pairs);
    let table2 = parse_table(pairs.next().unwrap());
    expect_token(Rule::KwOn, &mut pairs);
    let expr = parse_expr(pairs.next().unwrap());
    match kind.as_rule() {
        Rule::KwLeft => table1.left_join(table2, expr),
        Rule::KwInner => table1.inner_join(table2, expr),
        _ => unreachable!(),
    }
}

fn parse_expr(pair: Pair) -> Expr {
    match pair.as_rule() {
        Rule::CompoundIdent => Expr::col(pair.as_str()),
        Rule::ExprAnd => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                assert_eq!(op.as_rule(), Rule::KwAnd);
                let arg = parse_expr(pairs.next().unwrap());
                expr = expr.and(arg);
            }
            expr
        }
        Rule::ExprBitAnd => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                assert_eq!(op.as_rule(), Rule::OpBitAnd);
                let arg = parse_expr(pairs.next().unwrap());
                expr = expr & arg;
            }
            expr
        }
        Rule::ExprBitNot => {
            let mut pairs = pair.into_inner();
            expect_token(Rule::OpBitNot, &mut pairs);
            let arg = parse_expr(pairs.next().unwrap());
            expect_done(&mut pairs);
            arg.bitinv()
        }
        Rule::ExprBitOr => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                assert_eq!(op.as_rule(), Rule::OpBitOr);
                let arg = parse_expr(pairs.next().unwrap());
                expr = expr | arg;
            }
            expr
        }
        Rule::ExprCmp => {
            let mut pairs = pair.into_inner();
            let arg1 = parse_expr(pairs.next().unwrap());
            let op = pairs.next().unwrap().as_rule();
            let arg2 = parse_expr(pairs.next().unwrap());
            expect_done(&mut pairs);
            match op {
                Rule::OpEq => arg1.eq(arg2),
                Rule::OpGe => arg1.ge(arg2),
                Rule::OpGt => arg1.gt(arg2),
                Rule::OpLe => arg1.le(arg2),
                Rule::OpLt => arg1.lt(arg2),
                Rule::OpNeq => arg1.ne(arg2),
                _ => unreachable!(),
            }
        }
        Rule::ExprNeg => {
            let mut pairs = pair.into_inner();
            expect_token(Rule::OpMinus, &mut pairs);
            let arg = parse_expr(pairs.next().unwrap());
            expect_done(&mut pairs);
            -arg
        }
        Rule::ExprNot => {
            let mut pairs = pair.into_inner();
            expect_token(Rule::KwNot, &mut pairs);
            let arg = parse_expr(pairs.next().unwrap());
            expect_done(&mut pairs);
            arg.not()
        }
        Rule::ExprOr => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                assert_eq!(op.as_rule(), Rule::KwOr);
                let arg = parse_expr(pairs.next().unwrap());
                expr = expr.or(arg);
            }
            expr
        }
        Rule::ExprProd => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                let arg = parse_expr(pairs.next().unwrap());
                match op.as_rule() {
                    Rule::OpStar => {
                        expr = expr * arg;
                    }
                    Rule::OpSlash => {
                        expr = expr / arg;
                    }
                    _ => unreachable!(),
                }
            }
            expr
        }
        Rule::ExprShift => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                let arg = parse_expr(pairs.next().unwrap());
                match op.as_rule() {
                    Rule::OpShl => {
                        expr = expr << arg;
                    }
                    Rule::OpShr => {
                        expr = expr >> arg;
                    }
                    _ => unreachable!(),
                }
            }
            expr
        }
        Rule::ExprSum => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr(pairs.next().unwrap());
            while let Some(op) = pairs.next() {
                let arg = parse_expr(pairs.next().unwrap());
                match op.as_rule() {
                    Rule::OpPlus => {
                        expr = expr + arg;
                    }
                    Rule::OpMinus => {
                        expr = expr - arg;
                    }
                    _ => unreachable!(),
                }
            }
            expr
        }
        Rule::KwFalse => Expr::boolean(false),
        Rule::KwNull => Expr::null(),
        Rule::KwTrue => Expr::boolean(true),
        Rule::Integer => Expr::integer(parse_integer(pair)),
        Rule::String => Expr::string(parse_string(pair)),
        _ => {
            print_pair(pair, "");
            Expr::null()
        }
    }
}

fn parse_value(pair: Pair) -> Value {
    match pair.as_rule() {
        Rule::KwFalse => false.into(),
        Rule::KwNull => Value::Null,
        Rule::KwTrue => true.into(),
        Rule::Integer => Value::Int(parse_integer(pair)),
        Rule::String => Value::Str(parse_string(pair)),
        _ => unreachable!(),
    }
}

fn parse_integer(pair: Pair) -> i32 {
    assert_eq!(pair.as_rule(), Rule::Integer);
    pair.as_str().parse::<i32>().unwrap()
}

fn parse_string(pair: Pair) -> String {
    assert_eq!(pair.as_rule(), Rule::String);
    let inner = pair.into_inner().next().unwrap().as_str();
    let mut chars = inner.chars();
    let mut string = String::new();
    while let Some(ch) = chars.next() {
        string.push(if ch == '\\' {
            let esc = chars.next().unwrap();
            match esc {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'x' | 'u' => {
                    let mut hex = String::new();
                    hex.push(chars.next().unwrap());
                    hex.push(chars.next().unwrap());
                    if esc == 'u' {
                        hex.push(chars.next().unwrap());
                        hex.push(chars.next().unwrap());
                    }
                    let codepoint = u32::from_str_radix(&hex, 16).unwrap();
                    std::char::from_u32(codepoint)
                        .unwrap_or(std::char::REPLACEMENT_CHARACTER)
                }
                _ => esc,
            }
        } else {
            ch
        });
    }
    string
}

fn expect_done(pairs: &mut Pairs) {
    assert!(pairs.next().is_none());
}

fn expect_token<'a>(rule: Rule, pairs: &mut Pairs<'a>) -> Pair<'a> {
    let pair = pairs.next().unwrap();
    assert_eq!(pair.as_rule(), rule);
    pair
}

fn optional_token<'a>(rule: Rule, pairs: &mut Pairs<'a>) -> Option<Pair<'a>> {
    let opt_pair = pairs.next();
    if let Some(ref pair) = opt_pair {
        assert_eq!(pair.as_rule(), rule);
    }
    opt_pair
}

fn print_pairs(pairs: Pairs, indent: &str) {
    for pair in pairs {
        print_pair(pair, indent);
    }
}

fn print_pair(pair: Pair, indent: &str) {
    println!("{}{:?}", indent, pair.as_rule());
    print_pairs(pair.into_inner(), &format!("{indent}  "));
}

fn pad(mut string: String, fill: char, width: usize) -> String {
    while string.len() < width {
        string.push(fill);
    }
    string
}
