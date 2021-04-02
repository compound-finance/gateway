// TODO: Are we okay with this? XXX I think so?
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
    Amount(Amount),
    Max,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Chain {
    Eth,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Asset {
    Cash,
    Eth([u8; 20]),
}

#[derive(PartialEq, Eq, Debug)]
pub enum Account {
    Eth([u8; 20]),
}

#[derive(PartialEq, Eq, Debug)]
pub enum TrxRequest {
    Extract(MaxAmount, Asset, Account),
    Transfer(MaxAmount, Asset, Account),
    Liquidate(MaxAmount, Asset, Asset, Account),
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParseError<'a> {
    NotImplemented,
    LexError(&'a str),
    InvalidAmount,
    InvalidAddress,
    InvalidArgs(&'static str, usize, usize),
    UnknownFunction(&'a str),
    InvalidExpression,
    InvalidChain(&'a str),
    InvalidChainAccount(Chain),
}

fn parse_amount<'a>(t: &Token) -> Result<Amount, ParseError<'a>> {
    match t {
        Token::Integer(Some(v)) => Ok(*v),
        Token::Hex(Some(v)) => Ok(hex_util::hex_to_u128(v).ok_or(ParseError::InvalidAmount)?),
        _ => Err(ParseError::InvalidAmount), // TODO: Debug here?
    }
}

fn parse_max_amount<'a>(t: &Token) -> Result<MaxAmount, ParseError<'a>> {
    match t {
        Token::Identifier("Max") | Token::Identifier("MAX") => Ok(MaxAmount::Max),
        els => Ok(MaxAmount::Amount(parse_amount(els)?)),
    }
}

fn parse_chain<'a>(chain: &'a str) -> Result<Chain, ParseError<'a>> {
    match chain {
        "Eth" => Ok(Chain::Eth),
        _ => Err(ParseError::InvalidChain(chain)),
    }
}

fn parse_eth_address<'a>(account: &'a str) -> Result<[u8; 20], ParseError<'a>> {
    if &account[0..2] != "0x" {
        Err(ParseError::InvalidChainAccount(Chain::Eth))?;
    }

    let account_vec: Vec<u8> =
        hex::decode(&account[2..]).map_err(|_| ParseError::InvalidChainAccount(Chain::Eth))?;
    let chain_account: [u8; 20] = account_vec
        .try_into()
        .map_err(|_| ParseError::InvalidChainAccount(Chain::Eth))?;

    Ok(chain_account)
}

fn parse_chain_account<'a>(chain: Chain, address: &'a str) -> Result<Account, ParseError<'a>> {
    match chain {
        Chain::Eth => Ok(Account::Eth(parse_eth_address(address)?)),
    }
}

fn parse_chain_asset<'a>(chain: Chain, address: &'a str) -> Result<Asset, ParseError<'a>> {
    match chain {
        Chain::Eth => Ok(Asset::Eth(parse_eth_address(address)?)),
    }
}

fn parse_account<'a>(t: &Token<'a>) -> Result<Account, ParseError<'a>> {
    match t {
        Token::Pair(Some((chain_str, account_str))) => {
            let chain = parse_chain(chain_str)?;
            Ok(parse_chain_account(chain, account_str)?)
        }
        _ => Err(ParseError::InvalidAddress),
    }
}

fn parse_asset<'a>(t: &Token<'a>) -> Result<Asset, ParseError<'a>> {
    match t {
        Token::Identifier("Cash") | Token::Identifier("CASH") => Ok(Asset::Cash),
        Token::Pair(Some((chain_str, account_str))) => {
            let chain = parse_chain(chain_str)?;
            Ok(parse_chain_asset(chain, account_str)?)
        }
        _ => Err(ParseError::InvalidAddress),
    }
}

fn parse_extract<'a>(args: &[Token<'a>]) -> Result<TrxRequest, ParseError<'a>> {
    match args {
        [amount_token, asset_token, account_token] => {
            let max_amount = parse_max_amount(amount_token)?;
            let asset = parse_asset(asset_token)?;
            let account = parse_account(account_token)?;

            Ok(TrxRequest::Extract(max_amount, asset, account))
        }
        _ => Err(ParseError::InvalidArgs("Extract", 3, args.len())),
    }
}

fn parse_transfer<'a>(args: &[Token<'a>]) -> Result<TrxRequest, ParseError<'a>> {
    match args {
        [amount_token, asset_token, account_token] => {
            let max_amount = parse_max_amount(amount_token)?;
            let asset = parse_asset(asset_token)?;
            let account = parse_account(account_token)?;

            Ok(TrxRequest::Transfer(max_amount, asset, account))
        }
        _ => Err(ParseError::InvalidArgs("Transfer", 3, args.len())),
    }
}

fn parse_liquidate<'a>(args: &[Token<'a>]) -> Result<TrxRequest, ParseError<'a>> {
    match args {
        [amount_token, borrowed_asset_token, collateral_asset_token, account_token] => {
            let max_amount = parse_max_amount(amount_token)?;
            let borrowed_asset = parse_asset(borrowed_asset_token)?;
            let collateral_asset = parse_asset(collateral_asset_token)?;
            let account = parse_account(account_token)?;

            Ok(TrxRequest::Liquidate(
                max_amount,
                borrowed_asset,
                collateral_asset,
                account,
            ))
        }
        _ => Err(ParseError::InvalidArgs("Liquidate", 4, args.len())),
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
        [Token::LeftDelim, Token::Identifier("Extract"), args @ .., Token::RightDelim] => {
            parse_extract(args)
        }
        [Token::LeftDelim, Token::Identifier("Transfer"), args @ .., Token::RightDelim] => {
            parse_transfer(args)
        }
        [Token::LeftDelim, Token::Identifier("Liquidate"), args @ .., Token::RightDelim] => {
            parse_liquidate(args)
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
    const ETH: [u8; 20] = [
        238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238,
        238, 238,
    ];

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
        "(MyFun 3 Eth:0x55)" => Err(ParseError::UnknownFunction("MyFun")),
      parse_extract:
        "(Extract 3 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Extract(
          MaxAmount::Amount(3),
          Asset::Eth(ETH),
          Account::Eth(ALAN)
        )),
      parse_extract_cash_in_caps:
        "(Extract 3 CASH Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Extract(
          MaxAmount::Amount(3),
          Asset::Cash,
          Account::Eth(ALAN)
        )),
      parse_extract_cash_in_camel:
        "(Extract 3 Cash Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Extract(
          MaxAmount::Amount(3),
          Asset::Cash,
          Account::Eth(ALAN)
        )),
      parse_extract_hex:
        "(Extract 0x0100 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Extract(
          MaxAmount::Amount(256),
          Asset::Eth(ETH),
          Account::Eth(ALAN)
        )),
      parse_extract_max:
        "(Extract Max Cash Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Extract(
          MaxAmount::Max,
          Asset::Cash,
          Account::Eth(ALAN)
        )),
      parse_extract_max_caps:
        "(Extract MAX Cash Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Extract(
          MaxAmount::Max,
          Asset::Cash,
          Account::Eth(ALAN)
        )),
      parse_transfer:
        "(Transfer 3 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Transfer(
          MaxAmount::Amount(3),
          Asset::Eth(ETH),
          Account::Eth(ALAN)
        )),
      parse_transfer_max:
        "(Transfer Max Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Transfer(
          MaxAmount::Max,
          Asset::Eth(ETH),
          Account::Eth(ALAN)
        )),
      parse_liquidate_amount:
        "(Liquidate 55 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Cash Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Liquidate(
          MaxAmount::Amount(55),
          Asset::Eth(ETH),
          Asset::Cash,
          Account::Eth(ALAN)
        )),
      parse_liquidate_max:
        "(Liquidate Max Cash Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Ok(TrxRequest::Liquidate(
          MaxAmount::Max,
          Asset::Cash,
          Asset::Eth(ETH),
          Account::Eth(ALAN)
        )),
      // TODO: Should we prohibit non-Cash from being Maxable?
      parse_fail_no_zero_ex:
        "(Extract 3 Eth:xxeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidChainAccount(Chain::Eth)),
      parse_fail_invalid_amount_invalid:
        "(Extract hi Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidAmount),
      parse_fail_invalid_amount_too_large_int:
        "(Extract 340282366920938463463374607431768211456 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidAmount),
      parse_fail_invalid_amount_too_large_hex:
        "(Extract 0xffffffffffffffffffffffffffffffff00 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidAmount),
      parse_fail_invalid_asset:
        "(Extract 5 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeff Eth:0x0101010101010101010101010101010101010101)" => Err(ParseError::InvalidChainAccount(Chain::Eth)),
      parse_fail_invalid_recipient:
        "(Extract 5 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0101010101010101010101010101010101010101ff)" => Err(ParseError::InvalidChainAccount(Chain::Eth)),
    }
}
