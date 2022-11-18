use miette::SourceSpan;
use crate::IntegerType;

mod lexer;
mod parser;

pub trait Constraint {

}

impl Constraint for IntegerType {

}