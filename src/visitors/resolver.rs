use core::panic;
use std::collections::HashMap;

use crate::{
    ast::{
        Assign, Binary, Block, ClassDecl, Error, Expr, Expression, FunDecl, Get, Grouping,
        IVisitorExpr, IVisitorStmt, If, Literal, Logical, Print, Return, Set, Stmt, This, Unary,
        Var, VarDecl, While,
    },
    extract_enum_value,
};

use super::interpreter::Interpreter;

#[derive(Debug, PartialEq)]
pub struct Scope {
    symbol_table: HashMap<String, bool>,
    interpreter: Interpreter,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            symbol_table: HashMap::new(),
            interpreter: Interpreter::new(),
        }
    }

    pub fn define(&mut self, name: &str, value: bool) {
        self.symbol_table.insert(name.to_string(), value);
    }

    pub fn retrieve(&self, name: &str) -> Option<bool> {
        self.symbol_table.get(name).cloned()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.symbol_table.contains_key(name)
    }
}

pub struct Resolver<'a> {
    scopes: Vec<Scope>,
    interpreter: &'a mut Interpreter,
    current_function: FunctionType,
    current_class: ClassType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionType {
    None,
    Function,
    Method,
    Initializer,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ClassType {
    None,
    Class,
}

impl<'a> Resolver<'a> {
    pub fn new(interpreter: &'a mut Interpreter) -> Self {
        Resolver {
            scopes: vec![Scope::new()],
            interpreter,
            current_function: FunctionType::None,
            current_class: ClassType::None,
        }
    }

    pub fn begin_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn end_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, name: &str) -> Result<Option<Stmt>, Error> {
        if self.scopes.len() == 0 {
            return Ok(None);
        }
        let scope = self.scopes.last_mut().unwrap();
        if scope.exists(name) {
            return Err(Error::new(format!(
                "{:?} Already a variable with this name in this scope.",
                name
            )));
        }
        scope.define(name, false);
        Ok(None)
    }

    pub fn define(&mut self, name: &str) {
        if self.scopes.len() == 0 {
            return;
        }
        self.scopes.last_mut().unwrap().define(name, true);
    }

    //pub fn get(&self, name: &str) -> Option<bool> {
    //    self.environments.last().unwrap().retrieve(name)
    //}

    pub fn get(&self, name: &str) -> Option<bool> {
        for env in self.scopes.iter().rev() {
            let symbol = env.retrieve(name);
            if symbol.is_some() {
                return symbol;
            }
        }
        None
        //Err(Error::new(format!("{:?} is not defined.", name)))
    }

    /*pub fn resolve_local(&mut self, expr: &mut Expr, name: &str) {
        for (i, scope) in self.environments.iter().enumerate().rev() {
            //let pos = self.environments.len() - 1 - i;
            if scope.exists(name) {
                self.interpreter
                    .resolve(expr, self.environments.len() - 1 - i);
                return;
            }
        }
    }*/

    pub fn resolve_local(&mut self, expr: &Expr, name: &str) {
        for i in (0..self.scopes.len()).rev() {
            if let Some(_) = self.scopes[i].symbol_table.get(name) {
                self.interpreter.resolve(expr, self.scopes.len() - 1 - i);
                return;
            }
        }
    }

    pub fn resolve_function(
        &mut self,
        stmt: &Stmt,
        ftype: FunctionType,
    ) -> Result<Option<Stmt>, Error> {
        if let Stmt::FunDecl(FunDecl {
            name: _,
            parameters,
            body,
        }) = stmt
        {
            let enclosing_function = self.current_function.clone();
            self.current_function = ftype;
            self.begin_scope();

            for parameter in parameters {
                self.declare(&parameter.lexeme)?;
                self.define(&parameter.lexeme);
            }

            for stmt in body {
                stmt.accept(self).unwrap();
            }
            self.end_scope();
            self.current_function = enclosing_function;
            Ok(None)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }
}

impl<'a> IVisitorStmt<Result<Option<Stmt>, Error>> for Resolver<'a> {
    fn visit_expr(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        if let Stmt::Expression(Expression { expr }) = stmt {
            match expr.accept(self) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            }
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }

    fn visit_print(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        match stmt {
            Stmt::Print(Print { expr }) => match expr.accept(self) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            _ => Err(Error::new("Invalid statement".to_string())),
        }
    }

    fn visit_var_decl(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        match stmt {
            Stmt::VarDecl(VarDecl { name, expr }) => {
                self.declare(name.as_str())?;
                expr.accept(self).unwrap();
                self.define(name.as_str());
                Ok(None)
            }
            _ => Err(Error::new("Invalid statement".to_string())),
        }
    }

    fn visit_block(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        if let Stmt::Block(Block { stmts }) = stmt {
            self.begin_scope();
            for stmt in stmts {
                stmt.accept(self).unwrap();
            }
            self.end_scope();
            Ok(None)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }

    fn visit_if(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        match stmt {
            Stmt::If(If {
                condition,
                branch_true,
                branch_false,
            }) => {
                condition.accept(self).unwrap();
                branch_true.accept(self).unwrap();
                branch_false.accept(self).unwrap();
                Ok(None)
            }
            _ => Err(Error::new("Invalid statement".to_string())),
        }
    }

    fn visit_while(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        if let Stmt::While(While { condition, body }) = stmt {
            condition.accept(self).unwrap();
            body.accept(self).unwrap();
        }
        Ok(None)
    }

    fn visit_fun_decl(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        match stmt {
            Stmt::FunDecl(FunDecl {
                name,
                parameters: _,
                body: _,
            }) => {
                self.declare(name)?;
                self.define(name);

                self.resolve_function(stmt, FunctionType::Function)?;
                Ok(None)
            }
            _ => Err(Error::new("Invalid statement".to_string())),
        }
    }

    fn visit_return(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        match stmt {
            Stmt::Return(Return { keyword, value }) => {
                if self.current_function == FunctionType::None {
                    return Err(Error::new(format!(
                        "{:?} Can't return from top-level code.",
                        keyword
                    )));
                }
                if *value != Expr::Literal(Literal::Nil) {
                    if self.current_function == FunctionType::Initializer {
                        return Err(Error::new(format!(
                            "{:?} Can't return a value from an initializer.",
                            keyword
                        )));
                    }
                    value.accept(self).unwrap();
                }
                Ok(None)
            }
            _ => Err(Error::new("Invalid statement".to_string())),
        }
    }

    fn visit_class(&mut self, stmt: &Stmt) -> Result<Option<Stmt>, Error> {
        if let Stmt::ClassDecl(ClassDecl { name, methods }) = stmt {
            let enclosing_class = self.current_class.clone();
            self.current_class = ClassType::Class;
            self.declare(&name.lexeme)?;
            self.define(&name.lexeme);

            self.begin_scope();
            self.define("this");

            for method in methods {
                let fun_decl = extract_enum_value!(method, Stmt::FunDecl(c) => c);
                if fun_decl.name == "init" {
                    self.resolve_function(method, FunctionType::Initializer)?;
                } else {
                    self.resolve_function(method, FunctionType::Method)?;
                }
            }

            self.end_scope();
            self.current_class = enclosing_class;
            Ok(None)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }
}

impl<'a> IVisitorExpr<Result<Option<Expr>, Error>> for Resolver<'a> {
    fn visit_var(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Var(Var::Token(token)) = expr {
            if !(self.scopes.len() == 0) && (self.get(&token.lexeme).unwrap() == false) {
                Err(Error::new(format!(
                    "Can't read local variable in its own initializer."
                )))
            } else {
                self.resolve_local(&mut expr.clone(), &token.lexeme);
                Ok(None)
            }
        } else {
            panic!("Invalid expression");
        }
    }

    fn visit_literal(&mut self, _: &Expr) -> Result<Option<Expr>, Error> {
        Ok(None)
    }

    fn visit_unary(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Unary(Unary {
            operator: _operator,
            right,
        }) = expr
        {
            right.accept(self).unwrap();
            Ok(None)
        } else {
            Err(Error::new("Invalid expression".to_string()))
        }
    }

    fn visit_binary(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Binary(Binary {
            left,
            operator: _,
            right,
        }) = expr
        {
            left.accept(self).unwrap();
            right.accept(self).unwrap();
            Ok(None)
        } else {
            Err(Error::new("Invalid expression".to_string()))
        }
    }

    fn visit_grouping(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Grouping(Grouping { group }) = expr {
            group.accept(self)
        } else {
            Err(Error::new("Invalid expression".to_string()))
        }
    }

    fn visit_assign(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Assign(Assign { var, expr }) = expr {
            let Var::Token(token) = var;
            let _accepted_expr = expr.accept(self)?;
            self.resolve_local(&mut expr.clone(), &token.lexeme);
            Ok(None)
        } else {
            panic!("Invalid expression");
        }
    }

    fn visit_logical(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Logical(Logical {
            left,
            operator: _,
            right,
        }) = expr
        {
            left.accept(self).unwrap();
            right.accept(self).unwrap();
            Ok(None)
        } else {
            Err(Error::new("Invalid expression".to_string()))
        }
    }

    fn visit_call(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Call(call) = expr {
            call.callee.accept(self)?;
            let _: Vec<Expr> = call
                .arguments
                .iter()
                .map(|arg| arg.accept(self).unwrap().unwrap()) // ?
                .collect();
            Ok(None)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }

    fn visit_get(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Get(Get { object, name: _ }) = expr {
            object.accept(self)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }

    fn visit_set(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::Set(Set {
            object,
            name: _,
            value,
        }) = expr
        {
            object.accept(self)?;
            value.accept(self)?;
            Ok(None)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }

    fn visit_this(&mut self, expr: &Expr) -> Result<Option<Expr>, Error> {
        if let Expr::This(This { keyword }) = expr {
            if self.current_class == ClassType::None {
                return Err(Error::new(format!("Can't use 'this' outside of a class.")));
            }
            self.resolve_local(expr, keyword.lexeme.as_str());
            Ok(None)
        } else {
            Err(Error::new("Invalid statement".to_string()))
        }
    }
}