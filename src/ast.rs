// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

#[derive(Clone, Debug)]
pub enum Prog {
    Body{stmts: Block},
}

pub type Block = Vec<Stmt>;

#[derive(Clone, Debug)]
pub enum Stmt {
    Expr{expr: Expr},
}

pub type Expr = (RawExpr, (usize, usize));

#[derive(Clone, Debug)]
pub enum RawExpr {
    Null,

    Str{s: String},

    Var{name: String},

    Call{expr: Box<Expr>, args: Vec<Expr>},
}
