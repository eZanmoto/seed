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
    Declare{lhs: Expr, rhs: Expr},
    Assign{lhs: Expr, rhs: Expr},

    Expr{expr: Expr},
}

pub type Expr = (RawExpr, (usize, usize));

#[derive(Clone, Debug)]
pub enum RawExpr {
    Null,

    Bool{b: bool},
    Int{n: i64},
    Str{s: String},

    Var{name: String},

    List{items: Vec<Expr>},
    Object{props: Vec<PropItem>},

    Call{expr: Box<Expr>, args: Vec<Expr>},
}

#[derive(Clone,Debug)]
pub enum PropItem {
    Pair{name: Expr, value: Expr},
}
