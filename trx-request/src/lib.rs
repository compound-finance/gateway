#![feature(str_split_once)]
// TODO: Are we okay with this?
#![allow(incomplete_features)]
#![feature(unsized_locals)]

mod hex_util;
mod lex;
use lex::{lex, Token};
use logos::Lexer;
use std::convert::TryInto;

pub type Amount = u128;

#[derive(PartialEq, Eq, Debug)]
pub enum MaxAmount {
    Amt(Amount),
    Max,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Chain {
    Eth,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Account {
    Eth([u8; 20]),
}

#[derive(PartialEq, Eq, Debug)]
pub enum TrxRequest {
    MagicExtract(MaxAmount, Account),
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParseError<'a> {
    NotImplemented,
    LexError(&'a str),
    InvalidMaxAmount,
    InvalidAccount,
    InvalidArgs(&'static str, usize, usize),
    UnknownFunction(&'a str),
    InvalidExpression,
    InvalidChain(&'a str),
    InvalidChainAccount(Chain),
}

fn parse_max_amount<'a>(t: &Token) -> Result<MaxAmount, ParseError<'a>> {
    match t {
        Token::Integer(Some(v)) => Ok(MaxAmount::Amt(*v)),
        Token::Hex(Some(v)) => Ok(MaxAmount::Amt(
            hex_util::hex_to_u128(v).ok_or(ParseError::InvalidMaxAmount)?,
        )),
        Token::Identifier("max") => Ok(MaxAmount::Max),
        _ => Err(ParseError::InvalidMaxAmount), // TODO: Debug here?
    }
}

// TODO: How do handle casing here? For now, let's just assume all lower-case, which maybe we can enforce?
fn parse_chain<'a>(chain: &'a str) -> Result<Chain, ParseError<'a>> {
    match chain {
        "eth" => Ok(Chain::Eth),
        _ => Err(ParseError::InvalidChain(chain)),
    }
}

fn parse_chain_account<'a>(chain: Chain, account: &'a str) -> Result<Account, ParseError<'a>> {
    match chain {
        Chain::Eth => {
            let account_vec: Vec<u8> =
                hex::decode(&account[2..]).map_err(|_| ParseError::InvalidChainAccount(chain))?;
            let chain_account: [u8; 20] = account_vec
                .try_into()
                .map_err(|_| ParseError::InvalidChainAccount(Chain::Eth))?;
            Ok(Account::Eth(chain_account))
        }
    }
}

fn parse_account<'a>(t: &Token<'a>) -> Result<Account, ParseError<'a>> {
    match t {
        Token::Pair(Some((chain_str, account_str))) => {
            let chain = parse_chain(chain_str)?;
            Ok(parse_chain_account(chain, account_str)?)
        }
        _ => Err(ParseError::InvalidAccount),
    }
}

fn parse_magic_extract<'a>(args: &[Token<'a>]) -> Result<TrxRequest, ParseError<'a>> {
    match args {
        [amount_token, account_token] => {
            let amount = parse_max_amount(amount_token)?;
            let account = parse_account(account_token)?;

            Ok(TrxRequest::MagicExtract(amount, account))
        }
        _ => Err(ParseError::InvalidArgs("magic-extract", 2, args.len())),
    }
}

fn parse<'a>(tokens: Lexer<'a, Token<'a>>) -> Result<TrxRequest, ParseError<'a>> {
    // TODO: I don't love having to clone here at all
    tokens
        .clone()
        .spanned()
        .fold(Ok(()) as Result<(), ParseError<'a>>, |acc, el| {
            match (acc, el) {
                (Err(err), _) => Err(err),
                (_, (Token::Error, span)) => Err(ParseError::LexError(&tokens.source()[span])),
                (_, _) => Ok(()),
            }
        })?;

    let token_vec = tokens.collect::<Vec<Token<'a>>>();

    match &token_vec[..] {
        [Token::LeftDelim, Token::Identifier("magic-extract"), args @ .., Token::RightDelim] => {
            parse_magic_extract(args)
        }
        [Token::LeftDelim, Token::Identifier(fun), .., Token::RightDelim] => {
            Err(ParseError::UnknownFunction(fun))
        }
        _ => Err(ParseError::InvalidExpression),
    }
}

pub fn parse_request<'a>(request: &'a str) -> Result<TrxRequest, ParseError<'a>> {
    parse(lex(request))
}

#[cfg(test)]
mod tests {
    use crate::*;

    const ALAN: [u8; 20] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

    macro_rules! parse_tests {
    ($($name:ident: $input:expr => $exp:expr,)*) => {
    $(
        #[test]
        fn $name() {
          assert_eq!(
            $exp,
            parse_request($input)
          )
        }
    )*
    }
  }

    parse_tests! {
      parse_fail_lex_error:
        "(fricassée)" => Err(ParseError::LexError("é")),
      parse_fail_invalid_expression:
        "hello" => Err(ParseError::InvalidExpression),
      parse_fail_unknown_function:
        "(my-fun 3 eth:0x55)" => Err(ParseError::UnknownFunction("my-fun")),
      parse_fail_invalid_max:
        "(magic-extract mux eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidMaxAmount),
      parse_fail_invalid_max_too_large_int:
        "(magic-extract 340282366920938463463374607431768211456 eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidMaxAmount),
      parse_fail_invalid_max_too_large_hex:
        "(magic-extract 0xffffffffffffffffffffffffffffffff00 eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidMaxAmount),
      parse_simple_magic_extract:
        "(magic-extract 3 eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::MagicExtract(
          MaxAmount::Amt(3),
          Account::Eth(ALAN)
        )),
      parse_simple_hex_max:
        "(magic-extract 0x0100 eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::MagicExtract(
          MaxAmount::Amt(256),
          Account::Eth(ALAN)
        )),
      parse_simple_max_max:
        "(magic-extract max eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::MagicExtract(
          MaxAmount::Max,
          Account::Eth(ALAN)
        )),
    }
}
