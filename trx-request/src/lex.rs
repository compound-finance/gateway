use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq, Eq, Clone)]
pub enum Token<'a> {
    #[token("(")]
    LeftDelim,

    #[token(")")]
    RightDelim,

    #[regex(r"0x[0-9a-fA-F]+", parse_hex)]
    Hex(Option<Vec<u8>>),

    #[regex(r"[0-9]+", parse_int)]
    Integer(Option<u128>),

    #[regex(r"[a-zA-Z-]+")]
    Identifier(&'a str),

    #[regex(r"[a-zA-Z0-9]+:[a-zA-Z0-9]+", split_pair)]
    Pair(Option<(&'a str, &'a str)>),

    #[regex(r"[ \t\n\f]+", logos::skip)]
    Whitespace,

    #[error]
    Error,
}

fn split_pair<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Option<(&'a str, &'a str)> {
    lex.slice().split_once(':')
}

fn parse_hex<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Option<Vec<u8>> {
    hex::decode(&lex.slice()[2..]).ok()
}

fn parse_int<'a>(lex: &mut Lexer<'a, Token<'a>>) -> Option<u128> {
    u128::from_str_radix(&lex.slice()[..], 10).ok()
}

pub fn lex<'source>(text: &'source str) -> Lexer<'source, Token> {
    Token::lexer(text)
}

// TODO: handle lowercase
// TODO: handle odd-length hex numbers?
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! lex_tests {
    ($($name:ident: $input:expr => $exp:expr,)*) => {
    $(
        #[test]
        fn $name() {
          assert_eq!(
            $exp,
            lex($input).collect::<Vec<Token>>()
          )
        }
    )*
    }
  }

    lex_tests! {
      invalid_chars:
        "(hi!" => vec![Token::LeftDelim, Token::Identifier("hi"), Token::Error],
      invalid_unicode_chars:
        "(touché 50)" => vec![
          Token::LeftDelim,
          Token::Identifier("touch"),
          Token::Error,
          Token::Integer(Some(50)),
          Token::RightDelim
        ],
      simple_lex:
        "()" => vec![Token::LeftDelim, Token::RightDelim],
      simple_fun_call:
        "(my-fun 55 0x0100 eth:0x20)" => vec![
          Token::LeftDelim,
          Token::Identifier("my-fun"),
          Token::Integer(Some(55)),
          Token::Hex(Some(vec![1, 0])),
          Token::Pair(Some(("eth", "0x20"))),
          Token::RightDelim
        ],
    }
}
