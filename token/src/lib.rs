mod token;

use felico_base::result::FelicoResult;

pub use crate::token::{Token, TokenKind};

pub type TokenIterator<'source> = Box<dyn Iterator<Item = FelicoResult<Token<'source>>> + 'source>;
